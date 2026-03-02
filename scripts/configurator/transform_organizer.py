"""Phase 2a-2d: Transform organization, cleanup, and sample data extraction."""

import json
import os
import shutil
import sys

from utils.file_utils import read_json, write_json
from .constants import TRANSFORM_METADATA_FIELDS


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

    # Write sample_data.json
    if not config.dry_run:
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
