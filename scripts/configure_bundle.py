#!/usr/bin/env python3
"""Configure a Hydrolix integration deployment bundle.

Normalizes raw bundle assets (transforms, dashboards, summaries) into the
integration-deployment-templates format with template variables, cleaned
metadata, and a valid bundle.json.

Usage:
    python scripts/configure_bundle.py \
        --bundle-dir trafficpeak/bot-insights-cdn \
        --table-name bot_detection \
        --data-category security
"""

import argparse
import json
import os
import sys

# Add the scripts directory to the Python path for relative imports
SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPTS_DIR not in sys.path:
    sys.path.insert(0, SCRIPTS_DIR)

from configurator.config import BundleConfig, BundleState
from configurator.constants import VALID_DATA_CATEGORIES, VALID_FOLDERS, VALID_SUBFOLDERS
from configurator.discovery import run_discovery
from configurator.transform_organizer import run_transform_organization
from configurator.sql_analyzer import run_sql_analysis
from configurator.bundle_json_builder import run_bundle_json_build
from configurator.summary_fixer import run_summary_fix
from configurator.dashboard_fixer import run_dashboard_fix
from configurator.bundle_json_updater import run_bundle_json_update
from configurator.report import run_report

# Exit codes
EXIT_SUCCESS = 0
EXIT_ERROR = 1
EXIT_MISSING_INPUT = 2


def main():
    args = parse_args()

    # Load config from file if provided
    if args.config:
        config_data = _load_config_file(args.config)
        config = _build_config_from_dict(config_data, args)
    else:
        config = _build_config_from_args(args)

    # Validate required fields
    if not config.bundle_dir:
        print("Error: --bundle-dir is required", file=sys.stderr)
        sys.exit(EXIT_MISSING_INPUT)
    if not config.table_name:
        print("Error: --table-name is required", file=sys.stderr)
        sys.exit(EXIT_MISSING_INPUT)
    if not config.data_category:
        print("Error: --data-category is required", file=sys.stderr)
        sys.exit(EXIT_MISSING_INPUT)
    if config.data_category not in VALID_DATA_CATEGORIES:
        print(
            f"Error: --data-category must be one of: {', '.join(VALID_DATA_CATEGORIES)}",
            file=sys.stderr,
        )
        sys.exit(EXIT_MISSING_INPUT)
    if not all(c.isalnum() or c == '_' for c in config.table_name):
        print(
            f"Error: --table-name '{config.table_name}' is invalid - "
            "only letters, digits, and underscores allowed",
            file=sys.stderr,
        )
        sys.exit(EXIT_MISSING_INPUT)
    if config.folder and config.folder not in VALID_FOLDERS:
        print(
            f"Error: --folder '{config.folder}' is invalid. "
            f"Must be one of: {', '.join(VALID_FOLDERS)}",
            file=sys.stderr,
        )
        sys.exit(EXIT_MISSING_INPUT)
    if config.subfolder and not config.folder:
        print("Error: --subfolder requires --folder to be set", file=sys.stderr)
        sys.exit(EXIT_MISSING_INPUT)
    if config.subfolder and config.subfolder not in VALID_SUBFOLDERS.get(config.folder, ()):
        valid = VALID_SUBFOLDERS.get(config.folder, ())
        print(
            f"Error: --subfolder '{config.subfolder}' is invalid for folder '{config.folder}'. "
            f"Must be one of: {', '.join(valid) if valid else '(none)'}",
            file=sys.stderr,
        )
        sys.exit(EXIT_MISSING_INPUT)

    state = BundleState()

    if config.verbose:
        print(f"[Configure] Bundle dir: {config.bundle_dir}", file=sys.stderr)
        print(f"[Configure] Source: {config.source_name}", file=sys.stderr)
        print(f"[Configure] Bundle: {config.bundle_name}", file=sys.stderr)
        print(f"[Configure] Table: {config.table_name}", file=sys.stderr)
        print(f"[Configure] Prefix: {config.correct_prefix}_", file=sys.stderr)
        if config.dry_run:
            print("[Configure] DRY RUN - no files will be modified", file=sys.stderr)

    # Execute phases sequentially
    phases = [
        ("Phase 1: Discovery", run_discovery),
        ("Phase 2a-2d: Transform Organization", run_transform_organization),
        ("Phase 2e: SQL Analysis", run_sql_analysis),
        ("Phase 3: Build bundle.json", run_bundle_json_build),
        ("Phase 4: Fix Summaries", run_summary_fix),
        ("Phase 5: Fix Dashboards", run_dashboard_fix),
        ("Phase 6: Update bundle.json", run_bundle_json_update),
    ]

    for phase_name, phase_fn in phases:
        if config.verbose:
            print(f"\n[Configure] Starting {phase_name}...", file=sys.stderr)

        ok = phase_fn(config, state)
        if not ok:
            print(f"Error in {phase_name}: {'; '.join(state.errors)}", file=sys.stderr)
            # Still generate report on failure
            run_report(config, state)
            sys.exit(EXIT_ERROR)

    # Phase 7: Generate report
    run_report(config, state)

    if state.errors:
        sys.exit(EXIT_ERROR)
    sys.exit(EXIT_SUCCESS)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Configure a Hydrolix integration deployment bundle.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python scripts/configure_bundle.py \\
    --bundle-dir trafficpeak/bot-insights-cdn \\
    --table-name bot_detection \\
    --data-category security

  python scripts/configure_bundle.py \\
    --bundle-dir aws/my-bundle \\
    --table-name logs \\
    --data-category cdn \\
    --verbose --dry-run
        """,
    )

    # Required args (validated after config merge in main(), not by argparse,
    # so that --config can provide these values without CLI flags)
    parser.add_argument(
        "--bundle-dir",
        required=True,
        help="Path to bundle directory (relative to repo root or absolute)",
    )
    parser.add_argument(
        "--table-name",
        default="",
        help="Table name (e.g., logs, bot_detection). Required unless provided via --config.",
    )
    parser.add_argument(
        "--data-category",
        default="",
        help=f"Data category: {', '.join(VALID_DATA_CATEGORIES)}. Required unless provided via --config.",
    )

    # Optional args
    parser.add_argument(
        "--source-name",
        default="",
        help="Source name (default: inferred from path)",
    )
    parser.add_argument(
        "--bundle-name",
        default="",
        help="Bundle name (default: inferred from path)",
    )
    parser.add_argument(
        "--channel-type",
        default="",
        help="Channel type: AWS, Azure, GCP, 3rdParty, Internal (default: auto)",
    )
    parser.add_argument(
        "--maintainer",
        default="Hydrolix Team <team@hydrolix.io>",
        help="Maintainer contact",
    )
    parser.add_argument(
        "--description",
        default="",
        help="Bundle description (default: auto-generated)",
    )
    parser.add_argument(
        "--version",
        default="1.0.0",
        help="Bundle version (default: 1.0.0)",
    )
    parser.add_argument(
        "--method",
        default="",
        help="Override method detection (firehose, kinesis, http_streaming, multi_stream)",
    )
    parser.add_argument(
        "--primary-dashboard",
        default="",
        help="Primary dashboard filename (default: auto-detect)",
    )
    parser.add_argument(
        "--folder",
        default="",
        help=f"Grafana folder: {', '.join(VALID_FOLDERS)} (optional)",
    )
    parser.add_argument(
        "--subfolder",
        default="",
        help="Grafana subfolder (e.g., bots, ds2, siem, multi-cdn) (optional)",
    )
    parser.add_argument(
        "--beta",
        action="store_true",
        default=True,
        help="Mark bundle as beta (default: true)",
    )
    parser.add_argument(
        "--no-beta",
        action="store_true",
        default=False,
        help="Mark bundle as not beta",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        default=False,
        help="Print detailed progress to stderr",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        default=False,
        help="Show what would change without writing files",
    )
    parser.add_argument(
        "--config",
        default="",
        help="JSON config file (alternative to CLI args)",
    )

    return parser.parse_args()


def _build_config_from_args(args):
    """Build BundleConfig from parsed CLI arguments."""
    bundle_dir = args.bundle_dir

    # If relative path, resolve relative to repo root
    if not os.path.isabs(bundle_dir):
        # Try relative to CWD first
        if os.path.isdir(bundle_dir):
            bundle_dir = os.path.abspath(bundle_dir)
        else:
            # Try relative to repo root (parent of scripts/)
            repo_root = os.path.dirname(SCRIPTS_DIR)
            candidate = os.path.join(repo_root, bundle_dir)
            if os.path.isdir(candidate):
                bundle_dir = os.path.abspath(candidate)
            else:
                bundle_dir = os.path.abspath(bundle_dir)

    beta = args.beta and not args.no_beta

    return BundleConfig(
        bundle_dir=bundle_dir,
        table_name=args.table_name,
        data_category=args.data_category,
        source_name=args.source_name,
        bundle_name=args.bundle_name,
        channel_type=args.channel_type,
        maintainer=args.maintainer,
        description=args.description,
        version=args.version,
        method=args.method,
        primary_dashboard=args.primary_dashboard,
        beta=beta,
        verbose=args.verbose,
        dry_run=args.dry_run,
        folder=args.folder,
        subfolder=args.subfolder,
    )


def _load_config_file(config_path):
    """Load configuration from a JSON file."""
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, FileNotFoundError) as e:
        print(f"Error loading config file: {e}", file=sys.stderr)
        sys.exit(EXIT_MISSING_INPUT)


def _build_config_from_dict(data, args):
    """Build BundleConfig from a config dictionary, with CLI args as overrides."""
    bundle_dir = args.bundle_dir or data.get("bundle_dir", "")

    # Resolve path
    if bundle_dir and not os.path.isabs(bundle_dir):
        if os.path.isdir(bundle_dir):
            bundle_dir = os.path.abspath(bundle_dir)
        else:
            repo_root = os.path.dirname(SCRIPTS_DIR)
            candidate = os.path.join(repo_root, bundle_dir)
            if os.path.isdir(candidate):
                bundle_dir = os.path.abspath(candidate)
            else:
                bundle_dir = os.path.abspath(bundle_dir)

    beta = data.get("beta", True)
    if args.no_beta:
        beta = False

    return BundleConfig(
        bundle_dir=bundle_dir,
        table_name=args.table_name or data.get("table_name", ""),
        data_category=args.data_category or data.get("data_category", ""),
        source_name=args.source_name or data.get("source_name", ""),
        bundle_name=args.bundle_name or data.get("bundle_name", ""),
        channel_type=args.channel_type or data.get("channel_type", ""),
        maintainer=args.maintainer
        if args.maintainer != "Hydrolix Team <team@hydrolix.io>"
        else data.get("maintainer", "Hydrolix Team <team@hydrolix.io>"),
        description=args.description or data.get("description", ""),
        version=args.version if args.version != "1.0.0" else data.get("version", "1.0.0"),
        method=args.method or data.get("method", ""),
        primary_dashboard=args.primary_dashboard or data.get("primary_dashboard", ""),
        beta=beta,
        verbose=args.verbose or data.get("verbose", False),
        dry_run=args.dry_run or data.get("dry_run", False),
        folder=args.folder or data.get("folder", ""),
        subfolder=args.subfolder or data.get("subfolder", ""),
    )


if __name__ == "__main__":
    main()
