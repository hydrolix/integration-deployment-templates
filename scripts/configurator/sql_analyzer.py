"""Phase 2e: SQL analysis - parse SQL transforms, fix prefixes, collect dependencies."""

import os
import re
import sys

from utils.file_utils import read_json, write_json
from .constants import KNOWN_PREFIXES


# Regex to find function calls with known prefixes
# Matches: reference_breadcrumbs(, commons_city_name(, akamai_edge_worker(
FUNCTION_PATTERN = re.compile(
    r"(?<![a-zA-Z_])(" + "|".join(KNOWN_PREFIXES) + r")_([a-z][a-z0-9_]*)\("
)

# Regex to find dictGet calls with known prefixes.
# Matches: dictGet('reference_ua_cat_dict' and
# dictGetOrDefault('commons_geoip_asn_blocks_ipv4'
DICT_PATTERN = re.compile(
    r"dictGet(?:OrDefault)?\('((" + "|".join(KNOWN_PREFIXES) + r")_[a-z][a-z0-9_]*)'"
)

# Regex for replacing all known prefixes with the correct one
PREFIX_REPLACE_PATTERN = re.compile(
    r"(?<![a-zA-Z_])(" + "|".join(KNOWN_PREFIXES) + r")_(?=[a-z][a-z0-9_]*[\('])"
)


def run_sql_analysis(config, state):
    """Phase 2e: Analyze SQL transforms, fix prefixes, collect dependencies.

    For each transform:
    - Parse sql_transform for function/dictionary references
    - Replace incorrect prefixes with the correct one
    - Collect unique base names for shared_functions/shared_dictionaries
    """
    correct_prefix = config.correct_prefix
    all_functions = set()
    all_dictionaries = set()

    for tinfo in state.transforms:
        read_path = tinfo.final_path
        if config.dry_run:
            read_path = tinfo.original_path
        if not os.path.isfile(read_path):
            continue

        data = read_json(read_path)
        settings = data.get("settings", {})
        sql = settings.get("sql_transform")

        if not sql:
            tinfo.has_sql_transform = False
            if config.verbose:
                print(
                    f"[SQL Analysis] No sql_transform in "
                    f"{os.path.basename(tinfo.final_path)}, skipping",
                    file=sys.stderr,
                )
            continue

        tinfo.has_sql_transform = True

        # Find all function references
        func_matches = FUNCTION_PATTERN.findall(sql)
        for _prefix, base_name in func_matches:
            all_functions.add(base_name)
            tinfo.shared_functions.append(base_name)

        # Find all dictionary references
        dict_matches = DICT_PATTERN.findall(sql)
        for full_name, _prefix in dict_matches:
            all_dictionaries.add(full_name)
            tinfo.shared_dictionaries.append(full_name)

        # Replace prefixes in sql_transform
        new_sql = _replace_prefixes(sql, correct_prefix)

        if new_sql != sql:
            settings["sql_transform"] = new_sql
            if not config.dry_run:
                write_json(tinfo.final_path, data)
                if tinfo.final_path not in state.files_modified:
                    state.files_modified.append(tinfo.final_path)

            if config.verbose:
                print(
                    f"[SQL Analysis] Fixed prefixes in "
                    f"{os.path.basename(tinfo.final_path)} -> {correct_prefix}_",
                    file=sys.stderr,
                )

    # Store in state
    state.all_shared_functions = sorted(all_functions)
    state.all_shared_dictionaries = sorted(all_dictionaries)

    state.phases_completed.append("Phase 2e: SQL Analysis")

    if config.verbose:
        print(
            f"[SQL Analysis] Found {len(all_functions)} function(s), "
            f"{len(all_dictionaries)} dictionary(s)",
            file=sys.stderr,
        )

    return True


def _replace_prefixes(sql, correct_prefix):
    """Replace all known prefixes with the correct prefix in SQL text."""
    def replacer(match):
        return f"{correct_prefix}_"

    return PREFIX_REPLACE_PATTERN.sub(replacer, sql)
