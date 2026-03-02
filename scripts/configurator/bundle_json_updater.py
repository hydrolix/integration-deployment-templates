"""Phase 6: Update bundle.json with dashboard and summary paths."""

import os
import sys

from utils.file_utils import read_json, write_json


def run_bundle_json_update(config, state):
    """Phase 6: Add dashboard paths and summary tables to bundle.json.

    - Set dashboard.path to primary dashboard
    - Set other_dashboards[] for non-primary dashboards
    - Add summary_tables[] from Phase 4
    """
    bundle_dir = config.bundle_dir
    bundle_json_path = os.path.join(bundle_dir, "bundle.json")

    if not os.path.isfile(bundle_json_path):
        if config.dry_run:
            # In dry-run, bundle.json wasn't created - use empty dict
            data = {}
        else:
            state.errors.append("bundle.json not found - Phase 3 may have failed")
            return False
    else:
        data = read_json(bundle_json_path)

    # Update primary dashboard path
    if state.primary_dashboard:
        data["dashboard"] = {
            "path": f"dashboards/{state.primary_dashboard}",
            "project_var": "__PROJECT_NAME__",
        }

    # Update other dashboards
    other_dashboards = []
    for dinfo in state.dashboards:
        if not dinfo.is_primary:
            other_dashboards.append({
                "path": f"dashboards/{dinfo.filename}",
                "project_var": "__PROJECT_NAME__",
            })
    data["other_dashboards"] = other_dashboards

    # Update summary tables
    summary_tables = []
    for sinfo in state.summaries:
        summary_tables.append({
            "dashboard_var": sinfo.dashboard_var,
            "name": sinfo.name,
            "parent_table_name": config.table_name,
            "sql": {
                "path": f"summaries/{sinfo.filename}",
            },
        })
    data["summary_tables"] = summary_tables

    # Write
    if not config.dry_run:
        write_json(bundle_json_path, data)
        if bundle_json_path not in state.files_modified:
            state.files_modified.append(bundle_json_path)

    state.phases_completed.append("Phase 6: Update bundle.json")

    if config.verbose:
        print(
            f"[Bundle Update] Dashboard: {state.primary_dashboard}, "
            f"Other: {len(other_dashboards)}, "
            f"Summaries: {len(summary_tables)}",
            file=sys.stderr,
        )

    return True
