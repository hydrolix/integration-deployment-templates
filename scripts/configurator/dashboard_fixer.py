"""Phase 5: Fix dashboard structure - wrapper, UIDs, template vars, datasources."""

import json
import os
import re
import sys

from utils.file_utils import read_json, slugify_grafana_title, write_json
from .constants import (
    DASHBOARD_UUID_TEMPLATE,
    DATASOURCE_ELEMENT_MODEL,
    DATASOURCE_TEMPLATE,
    GRAFANA_SPECIAL_UIDS,
)


def run_dashboard_fix(config, state):
    """Phase 5: Fix all dashboard files.

    5a: Add dashboard wrapper, __elements, remove __inputs
    5b: Set uid to __DASHBOARD_UUID__
    5c: Fix template variables (primary vs other difference)
    5d: Replace all datasource UIDs with __DATASOURCE__
    5e: Rewrite hardcoded sibling/self dashboard UIDs to macros
    """
    if not state.dashboards:
        if config.verbose:
            print("[Dashboard Fix] No dashboards found, skipping Phase 5", file=sys.stderr)
        state.phases_completed.append("Phase 5: Fix Dashboards (skipped)")
        return True

    # Pre-build sibling slug map for step 5e (before the loop modifies any files)
    try:
        sibling_slug_map = _build_sibling_slug_map(state)
    except ValueError as exc:
        state.errors.append(str(exc))
        return False

    # Build summary var mapping from __inputs before removing them
    # We need to process all dashboards
    for dinfo in state.dashboards:
        if not os.path.isfile(dinfo.path):
            state.warnings.append(f"Dashboard file not found: {dinfo.path}")
            continue

        data = read_json(dinfo.path)

        # 5a: Fix wrapper and structure
        data = _fix_wrapper(data, config, state)

        # Get the dashboard content (might be wrapped or not)
        dashboard = data.get("dashboard", data)

        # Build __inputs mapping before removing
        inputs_map = _build_inputs_map(dashboard)

        # Remove __inputs
        if "__inputs" in dashboard:
            dashboard.pop("__inputs")

        # 5a: Fix __elements
        _fix_elements(dashboard)

        # 5b: Fix UID
        _fix_uid(dashboard)

        # 5c: Fix template variables
        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        # 5d: Fix datasource UIDs
        _fix_datasource_uids(dashboard)

        # 5e: Rewrite hardcoded sibling/self dashboard UIDs to macros
        _fix_hardcoded_uids(dashboard, dinfo, sibling_slug_map, config, state)

        # Remove id field
        if "id" in dashboard:
            dashboard["id"] = None

        # Write back
        if not config.dry_run:
            write_json(dinfo.path, data)
            state.files_modified.append(dinfo.path)

        if config.verbose:
            role = "primary" if dinfo.is_primary else "other"
            print(
                f"[Dashboard Fix] Fixed {dinfo.filename} ({role})",
                file=sys.stderr,
            )

    state.phases_completed.append("Phase 5: Fix Dashboards")
    return True


def _fix_wrapper(data, config, state):
    """Wrap dashboard content in {"dashboard": {...}} if needed."""
    if "dashboard" in data:
        return data

    # The entire data IS the dashboard - wrap it
    return {"dashboard": data}


def _fix_elements(dashboard):
    """Populate __elements with datasource model if empty."""
    elements = dashboard.get("__elements", {})
    if not elements or elements == {}:
        dashboard["__elements"] = DATASOURCE_ELEMENT_MODEL


def _fix_uid(dashboard):
    """Set dashboard uid to __DASHBOARD_UUID__."""
    dashboard["uid"] = DASHBOARD_UUID_TEMPLATE


def _build_inputs_map(dashboard):
    """Build a mapping from VAR_* names to their `value` from __inputs.

    The value (e.g. `akamai.edns_summary_hour`) is the author-bound table name
    and is the deterministic signal for classifying a VAR_* constant as a
    summary vs. raw-logs reference (LOTC-1449). Labels were used previously
    but are author-chosen and collide on substrings.

    Datasource inputs (no `value` field) are skipped; they're handled by
    _fix_datasource_uids.
    """
    inputs = dashboard.get("__inputs", [])
    mapping = {}
    for inp in inputs:
        name = inp.get("name", "")
        value = inp.get("value", "")
        if name.startswith("VAR_") and value:
            mapping[name] = value
    return mapping


def _fix_template_variables(dashboard, dinfo, inputs_map, config, state):
    """Fix templating variables based on primary vs other dashboard rules."""
    templating = dashboard.get("templating", {})
    var_list = templating.get("list", [])

    # Map summary names to their dashboard vars
    summary_var_map = {}
    for sinfo in state.summaries:
        summary_var_map[sinfo.name] = sinfo.dashboard_var

    new_var_list = []
    has_raw_table = False
    summary_vars_added = set()

    for var in var_list:
        var_name = var.get("name", "")
        var_type = var.get("type", "")
        query = var.get("query", "")

        # Check if this is a summary reference via ${VAR_*} pattern
        var_ref_match = re.match(r"^\$\{(VAR_[A-Z0-9_]+)\}$", query)

        if var_name == "raw_table":
            has_raw_table = True
            # Update it to correct format
            var["query"] = "__PROJECT_NAME__.__TABLE_NAME__"
            var["current"] = {
                "selected": False,
                "text": "__PROJECT_NAME__.__TABLE_NAME__",
                "value": "__PROJECT_NAME__.__TABLE_NAME__",
            }
            var["options"] = [{
                "selected": False,
                "text": "__PROJECT_NAME__.__TABLE_NAME__",
                "value": "__PROJECT_NAME__.__TABLE_NAME__",
            }]
            var["hide"] = 2
            var["type"] = "constant"
            var["skipUrlSync"] = True
            new_var_list.append(var)
            continue

        # Check for self-referencing VAR_TIMESTAMP constant
        if (
            var_ref_match
            and var_type == "constant"
            and var_name.lower() == "timestamp"
            and var_ref_match.group(1) == "VAR_TIMESTAMP"
        ):
            ts_col = _get_primary_timestamp_column(config, state)
            if ts_col:
                var["query"] = ts_col
                var["current"] = {
                    "selected": False,
                    "text": ts_col,
                    "value": ts_col,
                }
                var["options"] = [{
                    "selected": False,
                    "text": ts_col,
                    "value": ts_col,
                }]
                var["hide"] = 2
                var["skipUrlSync"] = True
            new_var_list.append(var)
            continue

        if var_ref_match and var_type == "constant":
            var_ref_name = var_ref_match.group(1)

            # LOTC-1449: classify by __inputs[VAR_X].value, not by variable
            # name. Variable names are author-chosen and collide on substrings
            # (e.g. `edns` as a substring of `edns_summary_hour`). Values in
            # __inputs are the table names the author bound at export time and
            # are deterministic.
            input_value = inputs_map.get(var_ref_name)
            resolved = _classify_input_value(
                input_value, config, state, dinfo.is_primary
            )
            # Fall back to the raw __inputs value when _classify_input_value
            # can't map it to a known placeholder (e.g. quantile SQL expressions
            # like `quantiles_response_ttfb_ms[2]`). Without this the
            # ${VAR_*} placeholder is left in the JSON and an older pipeline
            # step converted it to a self-referential Grafana variable reference
            # (${var_name}), causing infinite recursion in Grafana's variable
            # engine when the dashboard is opened for editing.
            effective = resolved if resolved is not None else input_value
            if effective is not None:
                var["query"] = effective
                var["current"] = {
                    "selected": False,
                    "text": effective,
                    "value": effective,
                }
                var["options"] = [{
                    "selected": False,
                    "text": effective,
                    "value": effective,
                }]
            new_var_list.append(var)
            continue

        if var_type == "adhoc":
            # Fix adhoc filter datasource
            ds = var.get("datasource", {})
            if isinstance(ds, dict) and ds.get("uid"):
                if ds["uid"] not in GRAFANA_SPECIAL_UIDS:
                    ds["uid"] = DATASOURCE_TEMPLATE
            new_var_list.append(var)
            continue

        # Keep other variables as-is
        new_var_list.append(var)

    # Add raw_table variable if not present
    if not has_raw_table:
        new_var_list.append({
            "current": {
                "selected": False,
                "text": "__PROJECT_NAME__.__TABLE_NAME__",
                "value": "__PROJECT_NAME__.__TABLE_NAME__",
            },
            "hide": 2,
            "name": "raw_table",
            "options": [{
                "selected": False,
                "text": "__PROJECT_NAME__.__TABLE_NAME__",
                "value": "__PROJECT_NAME__.__TABLE_NAME__",
            }],
            "query": "__PROJECT_NAME__.__TABLE_NAME__",
            "skipUrlSync": True,
            "type": "constant",
        })

    templating["list"] = new_var_list
    dashboard["templating"] = templating


def _get_primary_timestamp_column(config, state):
    """Read the first transform and return the name of the primary timestamp column.

    Walks settings.output_columns to find the column with datatype.primary == True.
    Returns the column name (e.g. "reqTimeSec", "timestamp"), or None if not found.
    """
    if not state.transforms:
        return None

    tinfo = state.transforms[0]
    read_path = tinfo.final_path
    if config.dry_run:
        read_path = tinfo.original_path

    if not read_path or not os.path.isfile(read_path):
        return None

    data = read_json(read_path)
    settings = data.get("settings", {})
    output_columns = settings.get("output_columns", [])

    for col in output_columns:
        datatype = col.get("datatype", {})
        if datatype.get("primary") is True:
            return col.get("name")

    return None


def _classify_input_value(value, config, state, is_primary):
    """Resolve a `__inputs[VAR_X].value` to its template placeholder, or None.

    The value is the table name the dashboard author bound to VAR_X (e.g.
    `akamai.edns_summary_hour`). A leading `<word>.` qualifier is stripped
    before comparison, so `akamai.foo`, `commons.foo`, and bare `foo` all
    behave the same.

    Resolution order (LOTC-1449):
      1. Matches a state.summaries[*].name → __SUMMARY_TABLE_NAME_N__
         (or `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_N__` for non-primary).
      2. Matches config.table_name → `__PROJECT_NAME__.__TABLE_NAME__`.
      3. Otherwise → None (caller leaves the variable untouched; mismatches
         surface via the Rust validator rather than being silently rewritten).
    """
    if not value:
        return None

    bare = value.split(".", 1)[1] if "." in value else value
    bare_lower = bare.lower()
    raw_table_lower = (config.table_name or "").lower()

    matched_summary_var = _find_summary_var(bare, state)

    # A summary name that equals the raw-table name is ambiguous — summary
    # wins resolution order, but flag it so the author can disambiguate rather
    # than chasing a downstream validator error with no pointer back.
    if matched_summary_var and bare_lower == raw_table_lower:
        state.warnings.append(
            f"Summary name and raw-table name collide on {bare!r} "
            f"(__inputs value {value!r}); routing to summary placeholder. "
            f"Rename the summary to disambiguate."
        )

    if matched_summary_var:
        return _get_summary_value(matched_summary_var, is_primary)

    if bare_lower == raw_table_lower:
        return "__PROJECT_NAME__.__TABLE_NAME__"

    return None


def _find_summary_var(label, state):
    """Find the __SUMMARY_TABLE_NAME_N__ var for a given summary name (exact match).

    Substring/endswith matching was removed in LOTC-1449: it mis-routed raw-table
    vars whose names were substrings of summary names (e.g. `edns` into
    `edns_summary_hour`). Callers should pass the already-resolved summary name
    (typically the __inputs value with any `<prefix>.` stripped).
    """
    label_lower = label.lower().strip()
    for sinfo in state.summaries:
        if sinfo.name.lower() == label_lower:
            return sinfo.dashboard_var
    return None


def _get_summary_value(dashboard_var, is_primary):
    """Get the correct summary variable value based on primary vs other."""
    if is_primary:
        # Primary dashboard: create_summary_table in Rust already prepends project_name
        return dashboard_var
    else:
        # Other dashboards: create_others replaces __SUMMARY_TABLE_NAME_N__ with bare name,
        # so __PROJECT_NAME__. prefix is needed here
        return f"__PROJECT_NAME__.{dashboard_var}"


def _fix_datasource_uids(dashboard):
    """Recursively replace all datasource UIDs with __DATASOURCE__."""
    _recursive_fix_uids(dashboard)


def _recursive_fix_uids(obj):
    """Recursively walk JSON and replace datasource UIDs."""
    if isinstance(obj, dict):
        # Check if this is a datasource object with a uid
        if "uid" in obj and "type" in obj:
            uid_val = obj["uid"]
            if isinstance(uid_val, str) and uid_val not in GRAFANA_SPECIAL_UIDS:
                obj["uid"] = DATASOURCE_TEMPLATE

        # Also check for standalone uid in datasource context
        if "uid" in obj and "datasource" not in obj:
            parent_has_datasource_type = obj.get("type", "").endswith("-datasource")
            if parent_has_datasource_type:
                uid_val = obj["uid"]
                if isinstance(uid_val, str) and uid_val not in GRAFANA_SPECIAL_UIDS:
                    obj["uid"] = DATASOURCE_TEMPLATE

        # Recurse into all values
        for key, val in obj.items():
            _recursive_fix_uids(val)

    elif isinstance(obj, list):
        for item in obj:
            _recursive_fix_uids(item)


# ---------------------------------------------------------------------------
# Step 5e: Hardcoded sibling/self dashboard UID rewrite
# ---------------------------------------------------------------------------

# Matches a 36-char lowercase hex UUID followed by / and a Grafana slug.
# Covers both Shape A (bare constant values) and Shape B (embedded in URLs).
_UID_SLUG_RE = re.compile(
    r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})"
    r"/"
    r"([a-z0-9][a-z0-9-]*)"
)


def _slug_to_macro(slug):
    """Convert a Grafana slug to its __DASHBOARD_UID_*__ macro name.

    Example: "raw-logs" -> "__DASHBOARD_UID_RAW_LOGS__"
    """
    return f"__DASHBOARD_UID_{slug.upper().replace('-', '_')}__"


def _rewrite_strings_in_json(obj, rewrite_fn):
    """Recursively walk a JSON structure and apply rewrite_fn to every string value in-place."""
    if isinstance(obj, dict):
        for key in obj:
            if isinstance(obj[key], str):
                obj[key] = rewrite_fn(obj[key])
            else:
                _rewrite_strings_in_json(obj[key], rewrite_fn)
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            if isinstance(item, str):
                obj[i] = rewrite_fn(item)
            else:
                _rewrite_strings_in_json(item, rewrite_fn)


def _build_sibling_slug_map(state):
    """Build a {slug: filename} map from all discovered dashboard titles.

    Raises ValueError on slug collision (two dashboards whose titles produce
    the same Grafana slug).
    """
    slug_map = {}
    seen = {}  # slug -> first filename seen

    for dinfo in state.dashboards:
        if not os.path.isfile(dinfo.path):
            continue
        try:
            data = read_json(dinfo.path)
        except Exception:
            continue
        dashboard = data.get("dashboard", data)
        title = dashboard.get("title", "")
        if not title:
            continue
        slug = slugify_grafana_title(title)
        if not slug:
            continue
        if slug in seen:
            raise ValueError(
                f"Slug collision in bundle: '{seen[slug]}' and '{dinfo.filename}' "
                f"both produce slug '{slug}'. Rename one dashboard title to disambiguate."
            )
        seen[slug] = dinfo.filename
        slug_map[slug] = dinfo.filename

    return slug_map


def _fix_hardcoded_uids(dashboard, dinfo, sibling_slug_map, config, state):
    """Detect and rewrite hardcoded <uuid>/<slug> patterns in dashboard JSON.

    Pass 1 (Shape A): walks templating.list[] for constant variables whose
    query value is a bare <uuid>/<slug>.  Rewrites to the appropriate macro and
    builds constants_by_target_slug for use in Pass 2.

    Pass 2 (Shape B): recursively walks all strings in the dashboard and
    rewrites any remaining <uuid>/<slug> occurrences, preferring
    ${constant_name} indirection when Pass 1 found a matching constant.

    Self-references (slug matches this dashboard's own title-slug) are rewritten
    to the existing __DASHBOARD_UUID__ macro.  Slugs not matching any sibling
    or self are left unchanged and a [WARN] is emitted.
    """
    title = dashboard.get("title", "")
    own_slug = slugify_grafana_title(title)

    # Pass 1 — Shape A: rewrite templating constants, build constant map
    constants_by_target_slug = {}  # sibling_slug -> constant_name

    templating = dashboard.get("templating", {})
    for var in templating.get("list", []):
        if var.get("type") != "constant":
            continue
        query = var.get("query", "")
        m = _UID_SLUG_RE.fullmatch(query)
        if not m:
            continue
        slug = m.group(2)

        if own_slug and slug == own_slug:
            new_uid_part = DASHBOARD_UUID_TEMPLATE
        elif slug in sibling_slug_map:
            new_uid_part = _slug_to_macro(slug)
            constants_by_target_slug[slug] = var["name"]
        else:
            state.warnings.append(
                f"{dinfo.filename}: hardcoded UID with unrecognized slug '{slug}' "
                f"in constant '{var.get('name', '')}' — may be an external dashboard link"
            )
            if config.verbose:
                print(
                    f"[WARN] {dinfo.filename}: hardcoded UID for unrecognized slug "
                    f"'{slug}' in constant '{var.get('name', '')}' — treating as external",
                    file=sys.stderr,
                )
            continue

        new_value = f"{new_uid_part}/{slug}"
        var["query"] = new_value
        if isinstance(var.get("current"), dict):
            var["current"]["value"] = new_value
            var["current"]["text"] = new_value
        for opt in var.get("options", []):
            if isinstance(opt, dict):
                opt["value"] = new_value
                opt["text"] = new_value

    # Pass 2 — Shape B: rewrite UID patterns in all strings (constants already
    # processed in Pass 1 no longer match the UUID regex so no double-rewrite).
    warned_slugs = set()

    def _rewrite(m):
        slug = m.group(2)
        if own_slug and slug == own_slug:
            return f"{DASHBOARD_UUID_TEMPLATE}/{slug}"
        if slug in sibling_slug_map:
            if slug in constants_by_target_slug:
                # Prefer Grafana variable indirection when a constant already exists
                return f"${{{constants_by_target_slug[slug]}}}"
            return f"{_slug_to_macro(slug)}/{slug}"
        # External / unrecognized slug — warn once per slug per dashboard
        if slug not in warned_slugs:
            warned_slugs.add(slug)
            state.warnings.append(
                f"{dinfo.filename}: hardcoded UID with unrecognized slug '{slug}' "
                f"in string — may be an external dashboard link"
            )
            if config.verbose:
                print(
                    f"[WARN] {dinfo.filename}: hardcoded UID for unrecognized slug "
                    f"'{slug}' in string — treating as external",
                    file=sys.stderr,
                )
        return m.group(0)

    _rewrite_strings_in_json(dashboard, lambda s: _UID_SLUG_RE.sub(_rewrite, s))
