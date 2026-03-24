#!/usr/bin/env python3
"""Pipeline orchestrator for Hydrolix integration deployment bundles.

Chains three stages in order:
1. bundle_to_yaml.py  — generate portable CaC YAML manifests (before in-place edits)
2. configure_bundle.py — configure raw bundle assets in-place
3. Rust validator      — validate the configured bundle

Usage:
    python scripts/run_pipeline.py \
        --bundle-dir trafficpeak/bot-insights-cdn \
        --table-name bot_detection \
        --data-category security
"""

import argparse
import json
import os
import subprocess
import sys
import time


SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPTS_DIR)

sys.path.insert(0, SCRIPTS_DIR)
from utils.file_utils import sanitize_cac_name
from configurator.config import is_semver, looks_like_version


def main():
    args = parse_args()

    # If --config provided, load it and fill in missing pipeline args so that
    # _require_stage2_args and build_*_cmd get the values without requiring
    # them on the CLI.  This lets CI run with just --config.
    if args.config:
        _merge_config_into_args(args)

    # Two-track pipeline: apply track selection before resolving stages
    _apply_track(args)

    stages = resolve_stages(args)

    results = []
    stage_num = 0
    total = len(stages)

    for name, build_fn in stages:
        stage_num += 1
        label = f"[Stage {stage_num}/{total}]"

        if name == "portable":
            cmd = build_portable_cmd(args)
            msg = "Generating portable bundle..."
        elif name == "configure":
            cmd = build_configure_cmd(args)
            msg = "Configuring bundle..."
        elif name == "validate":
            # Stage 2 report provides bundle_name for the validator filter
            stage2_report = _find_stage2_report(results)
            cmd = build_validate_cmd(args, stage2_report)
            msg = "Validating bundle..."
        else:
            continue

        if not args.json:
            print(f"\n{label} {msg}")

        parse_json = name == "configure"
        result = run_stage(name, cmd, REPO_ROOT, args.verbose, parse_json=parse_json)
        results.append(result)

        if result["status"] == "error":
            if not args.json:
                print(f"  ✗ FAILED")
                if result.get("stderr"):
                    for line in result["stderr"].strip().splitlines()[-10:]:
                        print(f"    {line}")
            print_summary(results, args)
            sys.exit(1)

        if not args.json:
            _print_stage_success(name, result)

    print_summary(results, args)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Pipeline orchestrator for Hydrolix integration deployment bundles.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Full pipeline
  python scripts/run_pipeline.py \\
      --bundle-dir trafficpeak/bot-insights-cdn \\
      --table-name bot_detection \\
      --data-category security

  # Skip validation
  python scripts/run_pipeline.py ... --skip-validate

  # Run only the validator
  python scripts/run_pipeline.py --bundle-dir trafficpeak/bot-insights-cdn --only-validate
        """,
    )

    # --- Shared / pipeline control ---
    parser.add_argument(
        "--bundle-dir", required=True,
        help="Path to bundle directory (relative to repo root)",
    )
    parser.add_argument("--verbose", action="store_true", help="Propagated to all stages")
    parser.add_argument("--dry-run", action="store_true",
                        help="Passed to stage 2; auto-skips stages 1 and 3")
    parser.add_argument("--json", action="store_true", help="Output final report as JSON")

    # --- Track selection (two-track pipeline) ---
    parser.add_argument(
        "--track", choices=["auto", "full", "validate-only"], default="auto",
        help="Pipeline track: 'auto' (detect from assets), 'full' (Stage 1+2+3), "
             "'validate-only' (Stage 3 only). Default: auto",
    )

    # --- Skip / only flags ---
    skip = parser.add_argument_group("stage selection (skip)")
    skip.add_argument("--skip-portable", action="store_true", help="Skip stage 1")
    skip.add_argument("--skip-configure", action="store_true", help="Skip stage 2")
    skip.add_argument("--skip-validate", action="store_true", help="Skip stage 3")

    only = parser.add_argument_group("stage selection (only)")
    only.add_argument("--only-portable", action="store_true", help="Run only stage 1")
    only.add_argument("--only-configure", action="store_true", help="Run only stage 2")
    only.add_argument("--only-validate", action="store_true", help="Run only stage 3")

    # --- Stage 1 passthrough (bundle_to_yaml.py) ---
    s1 = parser.add_argument_group("stage 1: bundle_to_yaml")
    s1.add_argument("--portable-output", default="",
                    help="Override portable output dir (default: portables/<bundle_name>/<version>/)")
    s1.add_argument("--skip-portable-validation", action="store_true",
                    help="Skip bundle_to_yaml's internal validation")
    s1.add_argument("--bundle-config", default="",
                    help="Path to bundle-config.json whose version must match bundle.json")
    s1.add_argument("--version", default="1.0.0", help="Bundle version (default: 1.0.0)")
    s1.add_argument("--maintainer", default="Hydrolix Team <team@hydrolix.io>",
                    help="Bundle maintainer")
    s1.add_argument("--description", default="", help="Bundle description")

    # --- Stage 2 passthrough (configure_bundle.py) ---
    s2 = parser.add_argument_group("stage 2: configure_bundle")
    s2.add_argument("--table-name", default="", help="Table name (required for stage 2)")
    s2.add_argument("--data-category", default="", help="Data category (required for stage 2)")
    s2.add_argument("--source-name", default="", help="Source name override")
    s2.add_argument("--bundle-name", default="", help="Bundle name override")
    s2.add_argument("--channel-type", default="", help="Channel type override")
    s2.add_argument("--method", default="", help="Method override")
    s2.add_argument("--primary-dashboard", default="", help="Primary dashboard filename")
    s2.add_argument("--beta", action="store_true", default=True, help="Mark as beta (default)")
    s2.add_argument("--no-beta", action="store_true", default=False, help="Mark as not beta")
    s2.add_argument("--config", default="", help="JSON config file for stage 2")

    # --- Stage 3 passthrough (Rust validator) ---
    s3 = parser.add_argument_group("stage 3: validator")
    s3.add_argument("--validator-wip", action="store_true", help="Pass --wip to validator")
    s3.add_argument("--validator-local", action="store_true", help="Pass --local to validator")
    s3.add_argument("--validator-strict-plugins", action="store_true",
                    help="Pass --strict-plugins to validator")
    s3.add_argument("--validator-production", action="store_true",
                    help="Pass --production to validator")

    return parser.parse_args()


def resolve_stages(args):
    """Determine which stages to run and validate required args."""
    only_flags = [args.only_portable, args.only_configure, args.only_validate]
    skip_flags = [args.skip_portable, args.skip_configure, args.skip_validate]

    if any(only_flags) and any(skip_flags):
        print("Error: cannot combine --only-* and --skip-* flags", file=sys.stderr)
        sys.exit(2)

    if sum(only_flags) > 1:
        print("Error: only one --only-* flag allowed", file=sys.stderr)
        sys.exit(2)

    # Dry-run: only stage 2 runs
    if args.dry_run:
        _require_stage2_args(args)
        return [("configure", build_configure_cmd)]

    # --only-* mode
    if args.only_portable:
        return [("portable", build_portable_cmd)]
    if args.only_configure:
        _require_stage2_args(args)
        return [("configure", build_configure_cmd)]
    if args.only_validate:
        return [("validate", build_validate_cmd)]

    # Default: all stages, minus any --skip-*
    stages = []
    if not args.skip_portable:
        stages.append(("portable", build_portable_cmd))
    if not args.skip_configure:
        _require_stage2_args(args)
        stages.append(("configure", build_configure_cmd))
    if not args.skip_validate:
        stages.append(("validate", build_validate_cmd))

    if not stages:
        print("Error: all stages skipped — nothing to do", file=sys.stderr)
        sys.exit(2)

    return stages


def _merge_config_into_args(args):
    """Load --config JSON and fill in missing pipeline args (table_name, data_category, etc.)."""
    config_path = args.config
    if not os.path.isabs(config_path):
        config_path = os.path.join(REPO_ROOT, config_path)

    try:
        with open(config_path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (json.JSONDecodeError, FileNotFoundError) as e:
        print(f"Warning: could not load config file '{args.config}': {e}", file=sys.stderr)
        return

    if not args.table_name:
        args.table_name = data.get("table_name", "")
    if not args.data_category:
        args.data_category = data.get("data_category", "")
    if not args.source_name:
        args.source_name = data.get("source_name", "")
    if not args.bundle_name:
        args.bundle_name = data.get("bundle_name", "")
    if not args.description:
        args.description = data.get("description", "")
    if args.version == "1.0.0":
        # Only override if still at default — config or path-based version takes precedence
        config_version = data.get("version", "")
        if config_version:
            args.version = config_version

    # Pre-check: config version must match folder name if both are set
    config_version = data.get("version", "")
    if config_version:
        dir_parts = args.bundle_dir.rstrip("/").split("/")
        folder_name = dir_parts[-1] if dir_parts else ""
        if is_semver(folder_name) and folder_name != config_version:
            print(
                f"Error: bundle-config.json version '{config_version}' does not match "
                f"folder name '{folder_name}'. They must be identical.",
                file=sys.stderr,
            )
            sys.exit(1)

    # Infer version from directory name if still at default and path is versioned
    if args.version == "1.0.0":
        parts = args.bundle_dir.rstrip("/").split("/")
        if parts and is_semver(parts[-1]):
            args.version = parts[-1]
        elif parts and looks_like_version(parts[-1]):
            print(
                f"Error: folder name '{parts[-1]}' looks like a version but is not valid "
                f"semver (expected X.Y.Z, e.g., 1.0.0). Rename the folder to a strict "
                f"X.Y.Z version or a plain bundle name.",
                file=sys.stderr,
            )
            sys.exit(1)


def _apply_track(args):
    """Apply two-track pipeline logic based on --track flag.

    - 'validate-only': forces --only-validate (Stage 3 only)
    - 'full': runs originals management, then full pipeline (default stages)
    - 'auto': detects track from bundle state, then applies accordingly
    """
    if args.track == "validate-only":
        args.only_validate = True
    elif args.track == "full":
        # Full pipeline — originals management happens here
        bundle_dir = os.path.join(REPO_ROOT, args.bundle_dir) if not os.path.isabs(args.bundle_dir) else args.bundle_dir
        from originals_manager import backup_to_originals, restore_from_originals
        originals_dir = os.path.join(REPO_ROOT, ".originals", os.path.relpath(bundle_dir, REPO_ROOT))
        if os.path.isdir(originals_dir):
            restore_from_originals(bundle_dir, REPO_ROOT)
        else:
            backup_to_originals(bundle_dir, REPO_ROOT)
    elif args.track == "auto":
        # Auto-detect: classify bundle state and route
        bundle_dir = os.path.join(REPO_ROOT, args.bundle_dir) if not os.path.isabs(args.bundle_dir) else args.bundle_dir
        from detect_track import classify_bundle_state
        originals_dir = os.path.join(REPO_ROOT, ".originals", os.path.relpath(bundle_dir, REPO_ROOT))
        has_originals = os.path.isdir(originals_dir)
        has_config = os.path.isfile(os.path.join(bundle_dir, "bundle-config.json"))

        state = classify_bundle_state(bundle_dir)

        if has_originals and state == "raw":
            # Raw assets detected with originals — treat as full
            args.track = "full"
            _apply_track(args)
        elif not has_originals and has_config and state == "raw":
            # First run — backup and run full
            args.track = "full"
            _apply_track(args)
        else:
            # Configured, ambiguous, or legacy — validate only
            args.track = "validate-only"
            _apply_track(args)


def _require_stage2_args(args):
    """Validate that stage 2 required args are present."""
    missing = []
    if not args.table_name:
        missing.append("--table-name")
    if not args.data_category:
        missing.append("--data-category")
    if missing:
        print(f"Error: {', '.join(missing)} required when stage 2 (configure) runs"
              " (or provide via --config)",
              file=sys.stderr)
        sys.exit(2)


def run_stage(name, cmd, cwd, verbose, parse_json=False):
    """Run a subprocess stage. Returns a result dict."""
    result = {"stage": name, "status": "success", "stdout": "", "stderr": "", "json": None,
              "duration": 0.0}
    start = time.time()

    try:
        if verbose:
            # Stream stderr in real-time, capture stdout
            proc = subprocess.Popen(
                cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=cwd,
            )
            stdout_lines = []
            # Read stderr line-by-line for real-time output
            import threading

            def _stream_stderr():
                for line in proc.stderr:
                    sys.stderr.write(f"  {line}")
                    sys.stderr.flush()

            t = threading.Thread(target=_stream_stderr, daemon=True)
            t.start()

            stdout, _ = proc.communicate(timeout=300)
            t.join(timeout=5)
            result["stdout"] = stdout
            result["stderr"] = ""  # already streamed
            returncode = proc.returncode
        else:
            completed = subprocess.run(
                cmd, capture_output=True, text=True, cwd=cwd, timeout=300,
            )
            result["stdout"] = completed.stdout
            result["stderr"] = completed.stderr
            returncode = completed.returncode

    except subprocess.TimeoutExpired:
        result["status"] = "error"
        result["stderr"] = f"Stage '{name}' timed out after 300 seconds"
        result["duration"] = time.time() - start
        return result
    except FileNotFoundError as e:
        result["status"] = "error"
        result["stderr"] = f"Command not found: {e}"
        result["duration"] = time.time() - start
        return result

    result["duration"] = time.time() - start

    if returncode != 0:
        result["status"] = "error"
        return result

    # Parse JSON from stdout if requested (stage 2)
    if parse_json and result["stdout"].strip():
        try:
            result["json"] = json.loads(result["stdout"])
        except json.JSONDecodeError:
            pass  # non-fatal — stage still succeeded

    return result


def build_portable_cmd(args):
    """Build command for stage 1: bundle_to_yaml.py."""
    cmd = [sys.executable, os.path.join(SCRIPTS_DIR, "bundle_to_yaml.py"),
           "--source", args.bundle_dir]

    if args.portable_output:
        cmd += ["--output", args.portable_output]
    else:
        # Default: portables/<customer_type>/<bundle_name>/<version>/
        parts = args.bundle_dir.rstrip("/").split("/")
        if is_semver(parts[-1]) and len(parts) >= 3:
            customer_type = sanitize_cac_name(parts[-3])
            bundle_name = sanitize_cac_name(parts[-2])
        elif looks_like_version(parts[-1]):
            print(
                f"Error: folder name '{parts[-1]}' looks like a version but is not valid "
                f"semver (expected X.Y.Z, e.g., 1.0.0). Rename the folder to a strict "
                f"X.Y.Z version or a plain bundle name.",
                file=sys.stderr,
            )
            sys.exit(1)
        else:
            customer_type = sanitize_cac_name(parts[-2]) if len(parts) >= 2 else sanitize_cac_name(parts[-1])
            bundle_name = sanitize_cac_name(parts[-1])
        output = os.path.join(REPO_ROOT, "portables", customer_type, bundle_name, args.version)
        cmd += ["--output", output]

    if args.version != "1.0.0":
        cmd += ["--version", args.version]
    if args.table_name:
        cmd += ["--table-name", args.table_name]
    if args.maintainer != "Hydrolix Team <team@hydrolix.io>":
        cmd += ["--maintainer", args.maintainer]
    if args.description:
        cmd += ["--description", args.description]
    if args.skip_portable_validation:
        cmd.append("--skip-validation")
    if args.bundle_config:
        cmd += ["--bundle-config", args.bundle_config]
    if args.verbose:
        cmd.append("--verbose")

    return cmd


def build_configure_cmd(args):
    """Build command for stage 2: configure_bundle.py."""
    cmd = [sys.executable, os.path.join(SCRIPTS_DIR, "configure_bundle.py"),
           "--bundle-dir", args.bundle_dir,
           "--table-name", args.table_name,
           "--data-category", args.data_category]

    if args.source_name:
        cmd += ["--source-name", args.source_name]
    if args.bundle_name:
        cmd += ["--bundle-name", args.bundle_name]
    if args.channel_type:
        cmd += ["--channel-type", args.channel_type]
    if args.maintainer != "Hydrolix Team <team@hydrolix.io>":
        cmd += ["--maintainer", args.maintainer]
    if args.description:
        cmd += ["--description", args.description]
    if args.version != "1.0.0":
        cmd += ["--version", args.version]
    if args.method:
        cmd += ["--method", args.method]
    if args.primary_dashboard:
        cmd += ["--primary-dashboard", args.primary_dashboard]
    if args.no_beta:
        cmd.append("--no-beta")
    if args.config:
        cmd += ["--config", args.config]
    if args.verbose:
        cmd.append("--verbose")
    if args.dry_run:
        cmd.append("--dry-run")

    return cmd


def build_validate_cmd(args, stage2_report=None):
    """Build command for stage 3: Rust validator (cargo run)."""
    cmd = ["cargo", "run", "--"]

    if args.validator_wip:
        cmd.append("--wip")
    if args.validator_local:
        cmd.append("--local")
    if args.validator_strict_plugins:
        cmd.append("--strict-plugins")
    if args.validator_production:
        cmd.append("--production")

    # Add bundle name filter
    bundle_filter = _get_bundle_filter(args, stage2_report)
    if bundle_filter:
        cmd.append(bundle_filter)

    return cmd


def _get_bundle_filter(args, stage2_report):
    """Get bundle name filter for the validator from stage 2 report or CLI args."""
    # Prefer stage 2 report if available
    if stage2_report and isinstance(stage2_report, dict):
        config = stage2_report.get("config", {})
        name = config.get("bundle_name", "")
        if name:
            return name

    # Fall back to --bundle-name arg or infer from --bundle-dir
    if args.bundle_name:
        return args.bundle_name
    return os.path.basename(args.bundle_dir.rstrip("/"))


def _find_stage2_report(results):
    """Extract stage 2 JSON report from results list."""
    for r in results:
        if r["stage"] == "configure" and r.get("json"):
            return r["json"]
    return None


def _print_stage_success(name, result):
    """Print human-readable success message for a completed stage."""
    duration = f"({result['duration']:.1f}s)"

    if name == "portable":
        print(f"  ✓ Portable bundle generated {duration}")
    elif name == "configure":
        report = result.get("json")
        if report:
            modified = len(report.get("files_modified", []))
            created = len(report.get("files_created", []))
            phases = len(report.get("phases_completed", []))
            print(f"  ✓ {phases} phases completed | {modified} files modified, {created} created {duration}")
        else:
            print(f"  ✓ Configuration complete {duration}")
    elif name == "validate":
        print(f"  ✓ Validation passed {duration}")


def print_summary(results, args):
    """Print final pipeline summary."""
    passed = sum(1 for r in results if r["status"] == "success")
    failed = sum(1 for r in results if r["status"] == "error")
    total = len(results)

    if args.json:
        summary = {
            "pipeline": "success" if failed == 0 else "error",
            "stages_run": total,
            "stages_passed": passed,
            "stages_failed": failed,
            "stages": [],
        }
        for r in results:
            entry = {"stage": r["stage"], "status": r["status"], "duration": r["duration"]}
            if r.get("json"):
                entry["report"] = r["json"]
            if r["status"] == "error" and r.get("stderr"):
                entry["error"] = r["stderr"].strip().splitlines()[-5:]
            summary["stages"].append(entry)
        print(json.dumps(summary, indent=2))
        return

    print()
    if failed == 0:
        print(f"Pipeline PASSED — all {total} stage(s) completed")
    else:
        failed_names = [r["stage"] for r in results if r["status"] == "error"]
        print(f"Pipeline FAILED — {', '.join(failed_names)} failed ({passed}/{total} passed)")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nPipeline cancelled by user.")
        sys.exit(130)
