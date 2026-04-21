"""Phase 2a-2d: Transform organization, cleanup, and sample data extraction."""

import calendar
import json
import os
import re
import shutil
import sys
import time
from datetime import datetime, timezone

from utils.file_utils import read_json, write_json
from .constants import TRANSFORM_METADATA_FIELDS

# 183 days in seconds (approximately 6 months)
_STALENESS_THRESHOLD_SECS = 183 * 86400

# Go reference time tokens -> strptime directives. Longer tokens first so that a
# positional scan picks "2006" before "06", "15" before "5", etc. Only padded
# tokens are supported; a layout using unpadded Go tokens (e.g. "1" for month,
# "5" for seconds) will produce a wrong strptime pattern and degrade to the
# warn-and-skip path in _shift_stale_datetime_primary.
_GO_LAYOUT_TOKENS = (
    ("2006", "%Y"),
    ("01", "%m"),
    ("02", "%d"),
    ("15", "%H"),
    ("04", "%M"),
    ("05", "%S"),
)


def _translate_go_layout(fmt):
    """Translate a Go reference-time layout to a strptime/strftime format string.

    Scans positionally so tokens never match inside strptime directives we've
    already emitted. Unknown characters pass through as literals.
    """
    out = []
    i = 0
    while i < len(fmt):
        matched = False
        for go_token, py_token in _GO_LAYOUT_TOKENS:
            if fmt.startswith(go_token, i):
                out.append(py_token)
                i += len(go_token)
                matched = True
                break
        if not matched:
            out.append(fmt[i])
            i += 1
    return "".join(out)


def run_transform_organization(config, state):
    """Phase 2a-2d: Organize, clean, and extract sample data from transforms.

    2a: Rename transforms/ -> transformations/, grafana/ -> dashboards/
    2b: Organize transform files into proper structure
    2c: Strip metadata fields
    2d: Extract and normalize sample data
    """
    bundle_dir = config.bundle_dir

    # Phase 2a: Normalize folder names
    _normalize_folders(bundle_dir, config, state)

    # Phase 2b: Organize transform files
    _organize_transforms(bundle_dir, config, state)

    if state.errors:
        return False

    # Phase 2c & 2d: Clean metadata and extract sample data
    for tinfo in state.transforms:
        # Determine where to read from: in dry-run, files haven't moved
        read_path = tinfo.final_path
        if config.dry_run:
            # In dry-run, use the original on-disk path
            read_path = tinfo.original_path

        if not os.path.isfile(read_path):
            state.errors.append(f"Transform file not found: {read_path}")
            return False

        data = read_json(read_path)

        # 2c: Strip metadata
        _clean_metadata(data, tinfo, config, state)

        # 2d: Extract and normalize sample data
        ok = _extract_sample_data(data, tinfo, config, state)
        if not ok:
            return False

        # Write cleaned transform
        if not config.dry_run:
            write_json(tinfo.final_path, data)
            state.files_modified.append(tinfo.final_path)

    state.phases_completed.append("Phase 2a-2d: Transform Organization")

    if config.verbose:
        print(
            f"[Transform Org] Organized {len(state.transforms)} transform(s)",
            file=sys.stderr,
        )

    return True


def _normalize_folders(bundle_dir, config, state):
    """Rename transforms/ -> transformations/ and grafana/ -> dashboards/."""
    transforms_src = os.path.join(bundle_dir, "transforms")
    transforms_dst = os.path.join(bundle_dir, "transformations")

    if os.path.isdir(transforms_src) and not os.path.isdir(transforms_dst):
        if not config.dry_run:
            shutil.move(transforms_src, transforms_dst)
            # Update original_path references since files actually moved
            for tinfo in state.transforms:
                tinfo.original_path = tinfo.original_path.replace(
                    transforms_src, transforms_dst
                )
        state.files_renamed.append(
            {"from": "transforms/", "to": "transformations/"}
        )
        if config.verbose:
            print("[Transform Org] Renamed transforms/ -> transformations/", file=sys.stderr)

    grafana_src = os.path.join(bundle_dir, "grafana")
    dashboards_dst = os.path.join(bundle_dir, "dashboards")

    if os.path.isdir(grafana_src) and not os.path.isdir(dashboards_dst):
        if not config.dry_run:
            shutil.move(grafana_src, dashboards_dst)
            # Update dashboard paths since files actually moved
            for d in state.dashboards:
                d.path = d.path.replace(grafana_src, dashboards_dst)
        if config.verbose:
            print("[Transform Org] Renamed grafana/ -> dashboards/", file=sys.stderr)


def _organize_transforms(bundle_dir, config, state):
    """Organize transform files into proper directory structure."""
    # Find the actual transforms directory on disk
    transformations_dir = os.path.join(bundle_dir, "transformations")
    transforms_dir = os.path.join(bundle_dir, "transforms")

    if os.path.isdir(transformations_dir):
        actual_dir = transformations_dir
    elif os.path.isdir(transforms_dir):
        actual_dir = transforms_dir  # In dry-run, rename didn't happen
    else:
        state.errors.append(
            f"transformations/ directory not found in {bundle_dir}"
        )
        return

    # The intended final directory is always "transformations/"
    final_base_dir = os.path.join(bundle_dir, "transformations")

    num_transforms = len(state.transforms)
    if num_transforms == 0:
        return

    if num_transforms == 1:
        _organize_single_transform(actual_dir, final_base_dir, config, state)
    else:
        _organize_multi_transforms(actual_dir, final_base_dir, config, state)


def _organize_single_transform(actual_dir, final_base_dir, config, state):
    """Handle single transform case - rename to transform.json if needed."""
    tinfo = state.transforms[0]
    target_path = os.path.join(final_base_dir, "transform.json")

    if os.path.basename(tinfo.original_path) != "transform.json":
        if not config.dry_run:
            actual_target = os.path.join(actual_dir, "transform.json")
            if os.path.dirname(tinfo.original_path) != actual_dir:
                shutil.move(tinfo.original_path, actual_target)
            else:
                os.rename(tinfo.original_path, actual_target)
        state.files_renamed.append(
            {"from": os.path.basename(tinfo.original_path), "to": "transform.json"}
        )
    # original_path stays as-is (for dry-run reads), final_path is the target
    tinfo.final_path = target_path
    tinfo.final_dir = final_base_dir
    tinfo.sample_data_path = os.path.join(final_base_dir, "sample_data.json")


def _organize_multi_transforms(actual_dir, final_base_dir, config, state):
    """Handle multi-provider transforms - create subdirectories."""
    for tinfo in state.transforms:
        provider = tinfo.provider_name
        if not provider:
            provider = "default"
            tinfo.provider_name = provider

        provider_dir = os.path.join(final_base_dir, provider)
        target_path = os.path.join(provider_dir, "transform.json")

        # Check if already organized at target
        if os.path.abspath(tinfo.original_path) == os.path.abspath(target_path):
            tinfo.final_path = target_path
            tinfo.final_dir = provider_dir
            tinfo.sample_data_path = os.path.join(provider_dir, "sample_data.json")
            continue

        if not config.dry_run:
            actual_provider_dir = os.path.join(actual_dir, provider)
            actual_target = os.path.join(actual_provider_dir, "transform.json")
            os.makedirs(actual_provider_dir, exist_ok=True)
            if os.path.isfile(tinfo.original_path):
                shutil.move(tinfo.original_path, actual_target)

        state.files_renamed.append(
            {
                "from": os.path.basename(tinfo.original_path),
                "to": f"{provider}/transform.json",
            }
        )

        # original_path stays as-is for dry-run reads; final_path is target
        tinfo.final_path = target_path
        tinfo.final_dir = provider_dir
        tinfo.sample_data_path = os.path.join(provider_dir, "sample_data.json")


def _clean_metadata(data, tinfo, config, state):
    """Strip metadata fields from transform data."""
    removed = []
    for fld in TRANSFORM_METADATA_FIELDS:
        if fld in data:
            data.pop(fld)
            removed.append(fld)

    if removed and config.verbose:
        print(
            f"[Transform Org] Removed metadata from "
            f"{os.path.basename(tinfo.final_path)}: {', '.join(removed)}",
            file=sys.stderr,
        )


def _extract_sample_data(data, tinfo, config, state):
    """Extract sample_data from transform and write to separate file."""
    settings = data.get("settings", {})
    sample_data = settings.get("sample_data")

    if sample_data is None or (isinstance(sample_data, list) and len(sample_data) == 0):
        state.errors.append(
            f"Transform '{os.path.basename(tinfo.final_path)}' is missing sample_data. "
            "Cannot proceed with bundle configuration."
        )
        return False

    # Normalize: if array, take first element
    if isinstance(sample_data, list):
        normalized = sample_data[0]
    elif isinstance(sample_data, dict):
        normalized = sample_data
    else:
        state.errors.append(
            f"Unexpected sample_data type in '{os.path.basename(tinfo.final_path)}': "
            f"{type(sample_data).__name__}"
        )
        return False

    # Update transform's sample_data to normalized single object
    settings["sample_data"] = normalized

    # Write sample_data.json (shift stale timestamps only on real runs)
    if not config.dry_run:
        _shift_stale_timestamps(normalized, data, tinfo, config)
        write_json(tinfo.sample_data_path, normalized)
        state.files_created.append(tinfo.sample_data_path)

    tinfo.has_sample_data = True

    if config.verbose:
        print(
            f"[Transform Org] Extracted sample_data -> "
            f"{os.path.basename(tinfo.sample_data_path)}",
            file=sys.stderr,
        )

    return True


_FORMAT_DIVISORS = {"s": 1, "ms": 1_000, "us": 1_000_000, "ns": 1_000_000_000}


def _resolve_sample_key(col, sample_data):
    """Resolve the actual key in sample_data for an output column.

    The output column name (e.g. "timestamp") may differ from the raw JSON
    input key (e.g. "reqTimeSec" or "EdgeStartTimestamp"). Prefer the output
    name if it is present with a non-null value; otherwise fall through to
    single-segment from_json_pointers and then from_input_field. As a last
    resort, return the output name even if its value is null so legacy
    callers still see the key.

    Only single-segment pointers (e.g. "/reqTimeSec") are resolved — nested
    pointers (e.g. "/avail/fillRate") cannot be mapped to a flat sample_data
    key and are skipped.
    """
    col_name = col.get("name")
    if not col_name:
        return None
    if col_name in sample_data and sample_data.get(col_name) is not None:
        return col_name

    source = col.get("datatype", {}).get("source") or {}
    for ptr in source.get("from_json_pointers", []):
        # Only resolve single-segment JSON pointers (e.g. "/reqTimeSec")
        segments = ptr.strip("/").split("/")
        if len(segments) == 1 and segments[0]:
            key = segments[0]
            if key in sample_data and sample_data.get(key) is not None:
                return key

    from_input = source.get("from_input_field")
    if (
        isinstance(from_input, str)
        and from_input in sample_data
        and sample_data.get(from_input) is not None
    ):
        return from_input

    if col_name in sample_data:
        return col_name
    return None


def _coerce_numeric_epoch(value):
    """Return value coerced to int if it's a numeric-looking string, else unchanged.

    Raw vendor exports sometimes serialize epochs as JSON strings (e.g.
    "1607368207"). Callers still want to treat them as numbers for the
    staleness check and delta math. Non-numeric values (including strings
    like "not-a-number") pass through unchanged.
    """
    if isinstance(value, str):
        stripped = value.strip()
        if stripped.removeprefix("-").isdigit():
            return int(stripped)
    return value


def _shift_stale_datetime_primary(sample_data, output_columns, tinfo, config):
    """Shift a stale datetime-typed primary timestamp to 1st-of-current-month UTC.

    Returns True if a datetime primary was found and processed (shifted or fresh),
    False if no datetime primary exists so the caller can fall through.

    Assumes sample_data has already been normalized to a dict by the upstream
    _extract_sample_data (which takes sample_data[0] for list-typed bundles).
    """
    primary_key = None
    primary_fmt = None
    for col in output_columns:
        dt = col.get("datatype", {})
        if dt.get("type") == "datetime" and dt.get("primary"):
            primary_key = _resolve_sample_key(col, sample_data)
            primary_fmt = dt.get("format", "")
            break

    if not primary_key or not primary_fmt:
        return False

    sample_value = sample_data.get(primary_key)
    if not isinstance(sample_value, str):
        return True

    strptime_fmt = _translate_go_layout(primary_fmt)
    try:
        parsed = datetime.strptime(sample_value, strptime_fmt).replace(tzinfo=timezone.utc)
    except (ValueError, re.error):
        print(
            f"[Transform Org] Unparseable datetime sample in "
            f"{os.path.basename(tinfo.final_path)}: format={primary_fmt!r}, "
            f"value={sample_value!r}",
            file=sys.stderr,
        )
        return True
    primary_secs = int(calendar.timegm(parsed.timetuple()))

    now_epoch = int(time.time())
    if now_epoch - primary_secs <= _STALENESS_THRESHOLD_SECS:
        return True

    now_utc = datetime.now(timezone.utc)
    first_of_month = now_utc.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
    sample_data[primary_key] = first_of_month.strftime(strptime_fmt)
    return True


def _shift_stale_timestamps(sample_data, transform_data, tinfo, config):
    """Shift stale epoch timestamps in sample_data to the 1st of the current month.

    Uses the transform's output_columns schema to identify epoch-typed fields.
    Only shifts if the primary timestamp is older than 6 months.
    Handles epoch formats: s, ms, us, ns.
    """
    output_columns = transform_data.get("settings", {}).get("output_columns", [])
    if not output_columns:
        return

    # Find the primary epoch column and build a map of epoch column formats
    # Resolve each column's actual key in sample_data (may differ from output name)
    primary_col = None
    primary_format = "s"
    epoch_columns = []  # list of (sample_key, format)
    for col in output_columns:
        dt = col.get("datatype", {})
        if dt.get("type") == "epoch":
            col_format = dt.get("format", "s")
            sample_key = _resolve_sample_key(col, sample_data)
            if sample_key:
                epoch_columns.append((sample_key, col_format))
            if dt.get("primary"):
                primary_col = (sample_key, col_format)
                primary_format = col_format

    if not primary_col or not primary_col[0]:
        if _shift_stale_datetime_primary(sample_data, output_columns, tinfo, config):
            return
        if config.verbose:
            print(
                f"[Transform Org] No primary epoch column found in "
                f"{os.path.basename(tinfo.final_path)}, skipping timestamp shift",
                file=sys.stderr,
            )
        return

    primary_key = primary_col[0]
    primary_value = _coerce_numeric_epoch(sample_data.get(primary_key))
    if not isinstance(primary_value, (int, float)):
        if config.verbose:
            print(
                f"[Transform Org] Primary timestamp '{primary_key}' is not numeric "
                f"in sample_data for {os.path.basename(tinfo.final_path)}, skipping shift",
                file=sys.stderr,
            )
        return

    # Convert to seconds for comparison regardless of format
    divisor = _FORMAT_DIVISORS.get(primary_format, 1)
    primary_secs = int(primary_value) // divisor

    now_epoch = int(time.time())
    staleness = now_epoch - primary_secs

    if staleness <= _STALENESS_THRESHOLD_SECS:
        return

    # Target: 1st of current month, midnight UTC
    now_utc = datetime.now(timezone.utc)
    first_of_month = now_utc.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
    target_epoch = int(calendar.timegm(first_of_month.timetuple()))

    # Delta in seconds (applied per-column in each column's native units)
    delta_secs = target_epoch - primary_secs

    shifted_fields = []
    for sample_key, col_format in epoch_columns:
        raw = sample_data.get(sample_key)
        coerced = _coerce_numeric_epoch(raw)
        if isinstance(coerced, (int, float)):
            col_divisor = _FORMAT_DIVISORS.get(col_format, 1)
            col_delta = delta_secs * col_divisor
            shifted = int(coerced) + col_delta
            sample_data[sample_key] = str(shifted) if isinstance(raw, str) else shifted
            shifted_fields.append(sample_key)

    if config.verbose and shifted_fields:
        print(
            f"[Transform Org] Shifted stale timestamps in "
            f"{os.path.basename(tinfo.sample_data_path)}: "
            f"{', '.join(shifted_fields)} (delta={delta_secs}s)",
            file=sys.stderr,
        )
