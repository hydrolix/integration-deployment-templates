"""Phase 4: Fix summary SQL files - replace hardcoded tables with template variables."""

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

    for idx, sinfo in enumerate(state.summaries, start=1):
        sinfo.dashboard_var = f"__SUMMARY_TABLE_NAME_{idx}__"

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
