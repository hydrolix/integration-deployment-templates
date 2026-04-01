"""Phase 5: Fix dashboard structure - wrapper, UIDs, template vars, datasources."""

import json
import os
import re
import sys

from utils.file_utils import read_json, write_json
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
    """
    if not state.dashboards:
        if config.verbose:
            print("[Dashboard Fix] No dashboards found, skipping Phase 5", file=sys.stderr)
        state.phases_completed.append("Phase 5: Fix Dashboards (skipped)")
        return True

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
    """Build a mapping from VAR_* names to their label/value from __inputs.

    Returns dict like: {"VAR_SUMMARY_HOUR": "summary_hour", ...}
    """
    inputs = dashboard.get("__inputs", [])
    mapping = {}
    for inp in inputs:
        name = inp.get("name", "")
        label = inp.get("label", "")
        if name.startswith("VAR_") and label:
            mapping[name] = label
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

        if var_ref_match and var_type == "constant":
            var_ref_name = var_ref_match.group(1)

            # Check if self-referencing: e.g. variable "table" with query "${VAR_TABLE}"
            stripped_name = var_ref_name[4:]  # Remove "VAR_" prefix
            if stripped_name.upper() == var_name.upper():
                # Self-referencing VAR_* constant — resolve to table placeholder
                # (timestamp is excluded here; handled separately in LOTC-1303)
                if var_name.lower() != "timestamp":
                    table_value = "__PROJECT_NAME__.__TABLE_NAME__"
                    var["query"] = table_value
                    var["current"] = {
                        "selected": False,
                        "text": table_value,
                        "value": table_value,
                    }
                    var["options"] = [{
                        "selected": False,
                        "text": table_value,
                        "value": table_value,
                    }]
                new_var_list.append(var)
                continue

            # This is a summary variable reference like ${VAR_SUMMARY_HOUR}
            label = inputs_map.get(var_ref_name, var_name)

            # Find matching summary by label/name
            matched_summary_var = _find_summary_var(label, state)
            if matched_summary_var:
                summary_value = _get_summary_value(
                    matched_summary_var, dinfo.is_primary
                )
                var["query"] = summary_value
                var["current"] = {
                    "selected": False,
                    "text": summary_value,
                    "value": summary_value,
                }
                var["options"] = [{
                    "selected": False,
                    "text": summary_value,
                    "value": summary_value,
                }]
                summary_vars_added.add(matched_summary_var)
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


def _find_summary_var(label, state):
    """Find the __SUMMARY_TABLE_NAME_N__ var for a given summary label/name."""
    label_lower = label.lower().strip()
    for sinfo in state.summaries:
        # Match by name or partial match
        if sinfo.name.lower() == label_lower:
            return sinfo.dashboard_var
        # Try matching summary_hour -> bot_summary_hour
        if label_lower in sinfo.name.lower() or sinfo.name.lower().endswith(label_lower):
            return sinfo.dashboard_var
    return None


def _get_summary_value(dashboard_var, is_primary):
    """Get the correct summary variable value based on primary vs other."""
    if is_primary:
        # Primary dashboard: NO __PROJECT_NAME__ prefix
        return dashboard_var
    else:
        # Other dashboards: WITH __PROJECT_NAME__ prefix
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
