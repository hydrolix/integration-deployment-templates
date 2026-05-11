"""Phase 3: Build bundle.json from discovered and analyzed assets."""

import os
import sys

from utils.file_utils import read_json, write_json
from .constants import METHOD_KEYWORDS, METHOD_UI, VALID_METHODS


def run_bundle_json_build(config, state):
    """Phase 3: Create or update bundle.json.

    - Detect method (single/multi_stream)
    - Build transform references with paths
    - Populate dependencies from SQL analysis
    - Merge with existing bundle.json if present
    """
    bundle_dir = config.bundle_dir
    bundle_json_path = os.path.join(bundle_dir, "bundle.json")

    # Detect method
    method = _detect_method(config, state)
    state.detected_method = method

    # Build the bundle.json structure
    bundle_data = _build_bundle_data(config, state, method)

    # Merge with existing if present
    if os.path.isfile(bundle_json_path):
        existing = read_json(bundle_json_path)
        bundle_data = _merge_with_existing(bundle_data, existing)
        if config.verbose:
            print("[Bundle JSON] Merged with existing bundle.json", file=sys.stderr)

    # Write
    if not config.dry_run:
        write_json(bundle_json_path, bundle_data)
        if os.path.isfile(bundle_json_path):
            state.files_modified.append(bundle_json_path)
        else:
            state.files_created.append(bundle_json_path)

    state.phases_completed.append("Phase 3: Build bundle.json")

    if config.verbose:
        print(
            f"[Bundle JSON] Method: {method}, "
            f"Transforms: {len(state.transforms)}, "
            f"Functions: {len(state.all_shared_functions)}, "
            f"Dictionaries: {len(state.all_shared_dictionaries)}",
            file=sys.stderr,
        )

    return True


def _detect_method(config, state):
    """Detect bundle method from transforms."""
    if config.method:
        return config.method

    num_transforms = len(state.transforms)

    if num_transforms > 1:
        return "multi_stream"

    if num_transforms == 1:
        tinfo = state.transforms[0]
        name = tinfo.provider_name.lower() if tinfo.provider_name else ""
        fname = os.path.basename(tinfo.original_path).lower()
        check_str = f"{name} {fname}"

        for keyword, method in METHOD_KEYWORDS.items():
            if keyword in check_str:
                return method

        return "http_streaming"

    return "http_streaming"


def _detect_transform_method(tinfo):
    """Detect method for a specific transform in multi_stream."""
    name = tinfo.provider_name.lower() if tinfo.provider_name else ""
    for keyword, method in METHOD_KEYWORDS.items():
        if keyword in name:
            return method
    return "http_streaming"


def _build_bundle_data(config, state, method):
    """Build the complete bundle.json data structure."""
    bundle_name = f"{config.source_name}_{config.bundle_name_normalized}"

    # Build transforms list
    transforms = []
    for tinfo in state.transforms:
        t_entry = {
            "path": _relative_path(config.bundle_dir, tinfo.final_path),
            "sample": _relative_path(config.bundle_dir, tinfo.sample_data_path),
        }

        if method == "multi_stream":
            t_method = _detect_transform_method(tinfo)
        else:
            t_method = method

        t_entry["method"] = t_method
        tinfo.method = t_method
        transforms.append(t_entry)

    # Method UI
    method_ui = METHOD_UI.get(
        method,
        METHOD_UI["http_streaming"],
    )

    # Source title — must be globally unique across all bundles
    bundle_label = config.bundle_name.replace("_", " ").replace("-", " ").title()
    source_label = config.source_name.replace("_", " ").replace("-", " ").title()
    source_title = f"{source_label} {bundle_label}"

    bundle_data = {
        "base_url": config.base_url,
        "beta": config.beta,
        "dashboard": {
            "path": f"dashboards/{state.primary_dashboard}" if state.primary_dashboard else "",
            "project_var": "__PROJECT_NAME__",
        },
        "dependencies": {
            "hydrolix": {
                "required_dictionaries": state.all_shared_dictionaries,
                "required_functions": [],
                "shared_dictionaries": [],
                "shared_functions": state.all_shared_functions,
            }
        },
        "metadata": {
            "channel_type": config.channel_type,
            "description": config.description,
            "maintainer": config.maintainer,
            "version": config.version,
        },
        "method": method,
        "name": bundle_name,
        "other_dashboards": [],
        "solution": True,
        "source": config.source_name,
        "summary_tables": [],
        "tables": [
            {
                "dashboard_var": "__TABLE_NAME__",
                "name": config.table_name,
                "transforms": transforms,
            }
        ],
        "ui": {
            "data_category": config.data_category,
            "method": method_ui,
            "primary_url": f"https://docs.hydrolix.io/docs/{config.bundle_name_normalized}-integration",
            "source": {
                "full_title": source_title,
                "icon_url": f"https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/{config.source_name}.png",
            },
        },
    }

    return bundle_data


def _merge_with_existing(new_data, existing):
    """Merge new bundle data with existing, preserving manual overrides."""
    # Preserve fields that may have been manually configured
    preserve_keys = ("method_overrides", "alert_rules")

    for key in preserve_keys:
        if key in existing and key not in new_data:
            new_data[key] = existing[key]

    # If existing has manual dependencies entries, merge them
    if "dependencies" in existing:
        ex_deps = existing["dependencies"]
        new_deps = new_data.get("dependencies", {})

        # Preserve grafana dependencies if present
        if "grafana" in ex_deps and "grafana" not in new_deps:
            new_deps["grafana"] = ex_deps["grafana"]

        # Preserve data-sources if present
        if "data-sources" in ex_deps and "data-sources" not in new_deps:
            new_deps["data-sources"] = ex_deps["data-sources"]

        new_data["dependencies"] = new_deps

    return new_data


def _relative_path(bundle_dir, abs_path):
    """Get path relative to bundle_dir."""
    return os.path.relpath(abs_path, bundle_dir)
