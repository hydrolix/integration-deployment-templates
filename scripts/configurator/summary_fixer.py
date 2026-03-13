"""Phase 4: Fix summary SQL files - replace hardcoded tables with template variables."""

import json
import os
import re
import sys

from utils.file_utils import write_file


# Pattern to match hardcoded schema.table references in FROM clauses
# Matches: FROM schema_name.table_name, FROM  schema.table
# Does NOT match already-templated references like __PROJECT_NAME__.__TABLE_NAME__
HARDCODED_TABLE_PATTERN = re.compile(
    r"(FROM\s+)(?!__PROJECT_NAME__\.)([a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_]*)",
    re.IGNORECASE,
)


def run_summary_fix(config, state):
    """Phase 4: Fix hardcoded table references in summary SQL files.

    - Replace hardcoded FROM schema.table with __PROJECT_NAME__.__TABLE_NAME__
    - Build summary_tables entries for bundle.json
    - Assign __SUMMARY_TABLE_NAME_N__ dashboard vars
    """
    if not state.summaries:
        if config.verbose:
            print("[Summary Fix] No summaries found, skipping Phase 4", file=sys.stderr)
        state.phases_completed.append("Phase 4: Fix Summaries (skipped)")
        return True

    # Preserve existing dashboard_var assignments from bundle.json
    existing_vars = {}
    bundle_json_path = os.path.join(config.bundle_dir, "bundle.json")
    if os.path.isfile(bundle_json_path):
        try:
            with open(bundle_json_path, "r", encoding="utf-8") as f:
                existing_bundle = json.load(f)
            for entry in existing_bundle.get("summary_tables", []):
                if "name" in entry and "dashboard_var" in entry:
                    existing_vars[entry["name"]] = entry["dashboard_var"]
        except Exception:
            pass

    used_nums = {
        int(v.replace("__SUMMARY_TABLE_NAME_", "").replace("__", ""))
        for v in existing_vars.values()
        if v.startswith("__SUMMARY_TABLE_NAME_") and v.endswith("__")
    }
    next_num = [n for n in range(1, len(state.summaries) + len(used_nums) + 2) if n not in used_nums]

    new_idx = 0
    for sinfo in state.summaries:
        if sinfo.name in existing_vars:
            sinfo.dashboard_var = existing_vars[sinfo.name]
        else:
            sinfo.dashboard_var = f"__SUMMARY_TABLE_NAME_{next_num[new_idx]}__"
            new_idx += 1

        # Read and fix SQL
        with open(sinfo.path, "r", encoding="utf-8") as f:
            sql_content = f.read()

        new_sql = _fix_table_references(sql_content)

        if new_sql != sql_content:
            if not config.dry_run:
                write_file(sinfo.path, new_sql)
            state.files_modified.append(sinfo.path)

            if config.verbose:
                print(
                    f"[Summary Fix] Fixed table references in {sinfo.filename}",
                    file=sys.stderr,
                )

    state.phases_completed.append("Phase 4: Fix Summaries")

    if config.verbose:
        print(
            f"[Summary Fix] Processed {len(state.summaries)} summary file(s)",
            file=sys.stderr,
        )

    return True


def _fix_table_references(sql_content):
    """Replace hardcoded schema.table with template variables."""
    return HARDCODED_TABLE_PATTERN.sub(
        r"\1__PROJECT_NAME__.__TABLE_NAME__", sql_content
    )
