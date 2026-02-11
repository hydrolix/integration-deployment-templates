"""Phase 7: Generate structured configuration report."""

import json
import os
import sys


def run_report(config, state):
    """Phase 7: Generate structured JSON report to stdout."""
    report = {
        "status": "success" if not state.errors else "error",
        "bundle_dir": config.bundle_dir,
        "config": {
            "source_name": config.source_name,
            "bundle_name": config.bundle_name,
            "table_name": config.table_name,
            "data_category": config.data_category,
            "method": state.detected_method,
            "channel_type": config.channel_type,
            "beta": config.beta,
            "dry_run": config.dry_run,
        },
        "phases_completed": state.phases_completed,
        "transforms": [
            {
                "provider": t.provider_name or "(root)",
                "path": os.path.relpath(t.final_path, config.bundle_dir)
                if t.final_path
                else t.original_path,
                "method": t.method,
                "has_sql": t.has_sql_transform,
                "has_sample_data": t.has_sample_data,
            }
            for t in state.transforms
        ],
        "dashboards": {
            "primary": state.primary_dashboard,
            "other": [
                d.filename for d in state.dashboards if not d.is_primary
            ],
        },
        "summaries": [
            {
                "name": s.name,
                "dashboard_var": s.dashboard_var,
                "file": s.filename,
            }
            for s in state.summaries
        ],
        "dependencies": {
            "shared_functions": state.all_shared_functions,
            "shared_dictionaries": state.all_shared_dictionaries,
        },
        "files_created": [
            os.path.relpath(f, config.bundle_dir) for f in state.files_created
        ],
        "files_modified": [
            os.path.relpath(f, config.bundle_dir) for f in state.files_modified
        ],
        "files_renamed": state.files_renamed,
        "warnings": state.warnings,
        "errors": state.errors,
    }

    state.phases_completed.append("Phase 7: Report")

    # Output report as JSON to stdout
    print(json.dumps(report, indent=2))

    return True
