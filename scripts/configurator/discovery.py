"""Phase 1: Discovery - scan bundle directory and catalog assets."""

import os
import sys

from .config import BundleState, DashboardInfo, SummaryInfo, TransformInfo
from .constants import PRIMARY_DASHBOARD_NAMES


def run_discovery(config, state):
    """Phase 1: Discover and catalog all bundle assets.

    Scans the bundle directory for transforms, dashboards, and summaries.
    Populates BundleState with discovered assets.
    """
    bundle_dir = config.bundle_dir

    if not os.path.isdir(bundle_dir):
        state.errors.append(f"Bundle directory does not exist: {bundle_dir}")
        return False

    # Discover transforms
    _discover_transforms(bundle_dir, state)

    # Discover dashboards
    _discover_dashboards(bundle_dir, config, state)

    # Discover summaries
    _discover_summaries(bundle_dir, state)

    # Validate minimum requirements
    if not state.transforms:
        state.errors.append(
            f"No transform files found in {bundle_dir}. "
            "Expected transforms/ or transformations/ directory with JSON files."
        )
        return False

    state.phases_completed.append("Phase 1: Discovery")

    if config.verbose:
        print(
            f"[Discovery] Found {len(state.transforms)} transform(s), "
            f"{len(state.dashboards)} dashboard(s), "
            f"{len(state.summaries)} summary(s)",
            file=sys.stderr,
        )

    return True


def _discover_transforms(bundle_dir, state):
    """Find all transform JSON files."""
    # Check for transforms/ or transformations/ directory
    transforms_dir = None
    for name in ("transforms", "transformations"):
        candidate = os.path.join(bundle_dir, name)
        if os.path.isdir(candidate):
            transforms_dir = candidate
            break

    if not transforms_dir:
        return

    # Look for JSON files (transforms) - could be at root or in subdirectories
    for item in sorted(os.listdir(transforms_dir)):
        item_path = os.path.join(transforms_dir, item)

        if os.path.isfile(item_path) and item.endswith(".json"):
            # Skip sample_data.json files
            if item.lower() == "sample_data.json":
                continue
            info = TransformInfo(original_path=item_path)
            info.provider_name = _extract_provider_name(item)
            state.transforms.append(info)

        elif os.path.isdir(item_path):
            # Check for transform.json in subdirectory
            for subitem in sorted(os.listdir(item_path)):
                subitem_path = os.path.join(item_path, subitem)
                if (
                    os.path.isfile(subitem_path)
                    and subitem.endswith(".json")
                    and subitem.lower() != "sample_data.json"
                ):
                    info = TransformInfo(original_path=subitem_path)
                    info.provider_name = item
                    state.transforms.append(info)


def _discover_dashboards(bundle_dir, config, state):
    """Find all dashboard JSON files."""
    # Check for dashboards/ or grafana/ directory
    dashboards_dir = None
    for name in ("dashboards", "grafana"):
        candidate = os.path.join(bundle_dir, name)
        if os.path.isdir(candidate):
            dashboards_dir = candidate
            break

    if not dashboards_dir:
        return

    dashboard_files = sorted([
        f
        for f in os.listdir(dashboards_dir)
        if f.endswith(".json") and os.path.isfile(os.path.join(dashboards_dir, f))
    ])

    for fname in dashboard_files:
        fpath = os.path.join(dashboards_dir, fname)
        info = DashboardInfo(path=fpath, filename=fname)
        state.dashboards.append(info)

    # Determine primary dashboard
    if config.primary_dashboard:
        # User specified
        for d in state.dashboards:
            if d.filename == config.primary_dashboard:
                d.is_primary = True
                state.primary_dashboard = d.filename
                break
        if not state.primary_dashboard:
            state.warnings.append(
                f"Specified primary dashboard '{config.primary_dashboard}' not found"
            )
    else:
        # Auto-detect
        _auto_detect_primary(state)


def _auto_detect_primary(state):
    """Auto-detect the primary dashboard by name priority or single file."""
    if not state.dashboards:
        return

    # Check priority names
    for name in PRIMARY_DASHBOARD_NAMES:
        for d in state.dashboards:
            if d.filename.lower() == name.lower():
                d.is_primary = True
                state.primary_dashboard = d.filename
                return

    # If only one dashboard, it's the primary
    if len(state.dashboards) == 1:
        state.dashboards[0].is_primary = True
        state.primary_dashboard = state.dashboards[0].filename
        return

    # Default to first file alphabetically
    state.dashboards[0].is_primary = True
    state.primary_dashboard = state.dashboards[0].filename
    state.warnings.append(
        f"Multiple dashboards found, auto-selected '{state.primary_dashboard}' "
        f"as primary. Use --primary-dashboard to override."
    )


def _discover_summaries(bundle_dir, state):
    """Find all summary SQL files."""
    summaries_dir = os.path.join(bundle_dir, "summaries")
    if not os.path.isdir(summaries_dir):
        return

    for fname in sorted(os.listdir(summaries_dir)):
        if fname.endswith(".sql") and os.path.isfile(
            os.path.join(summaries_dir, fname)
        ):
            info = SummaryInfo(
                path=os.path.join(summaries_dir, fname),
                filename=fname,
                name=fname.replace(".sql", ""),
            )
            state.summaries.append(info)


def _extract_provider_name(filename):
    """Extract provider name from transform filename.

    Examples:
        'akamai (4).json' -> 'akamai'
        'cloudfront_firehose.json' -> 'cloudfront_firehose'
        'default (11).json' -> 'default'
        'transform.json' -> ''
    """
    name = filename.replace(".json", "")
    # Strip parenthetical numbers and whitespace
    name = name.strip()
    # Remove trailing (N) pattern
    import re

    name = re.sub(r"\s*\(\d+\)\s*$", "", name)
    name = name.strip()

    if name.lower() == "transform":
        return ""
    return name
