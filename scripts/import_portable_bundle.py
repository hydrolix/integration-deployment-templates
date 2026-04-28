#!/usr/bin/env python3
"""Import an existing CaC bundle into the portable bundle tree.

This is for bundles that are already in CaC shape, for example:

    data/bundles/trafficpeak/bot_insights_cdn/1.1.9/

It copies the bundle into:

    portables/<customer_type>/<bundle_name>/<version>/

and vendors known cac-tools default resources that would otherwise be referenced
through relative __extend__ paths outside this repository.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from pathlib import Path


DEFAULT_CAC_TOOLS_ROOT = Path.home() / "src" / "cac-tools"

VENDORED_EXTENDS = {
    "hydrolix/_defaults/resources/transforms/transform_akamai_default_datastream.yaml": {
        "replacement": "transforms/akamai_datastream2_transform.json",
        "copies": [
            (
                "data/hydrolix/_defaults/integrations/akamai-cdn/transforms/akamai_datastream2_transform.json",
                "hydrolix/transforms/akamai_datastream2_transform.json",
            ),
        ],
    },
    "hydrolix/_defaults/table_summary_base_defaults.yaml": {
        "replacement": "table_summary_base_defaults.yaml",
        "copies": [
            (
                "data/hydrolix/_defaults/table_summary_base_defaults.yaml",
                "hydrolix/tables/table_summary_base_defaults.yaml",
            ),
            (
                "data/hydrolix/_defaults/table_defaults.yaml",
                "hydrolix/tables/table_defaults.yaml",
            ),
        ],
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Import a generated CaC bundle into portables/."
    )
    parser.add_argument(
        "source",
        type=Path,
        help="Source CaC bundle directory, e.g. ~/src/cac-tools/data/bundles/trafficpeak/name/version",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="integration-deployment-templates repo root",
    )
    parser.add_argument(
        "--cac-tools-root",
        type=Path,
        default=DEFAULT_CAC_TOOLS_ROOT,
        help="cac-tools repo root used for vendored defaults",
    )
    parser.add_argument("--customer-type", help="Override customer type")
    parser.add_argument("--bundle-name", help="Override bundle name")
    parser.add_argument("--version", help="Override bundle version")
    parser.add_argument(
        "--force",
        action="store_true",
        help="Replace the destination directory if it already exists",
    )
    parser.add_argument(
        "--skip-validation",
        action="store_true",
        help="Skip portable bundle output validation",
    )
    return parser.parse_args()


def resolve_source(path: Path) -> Path:
    resolved = path.expanduser().resolve()
    if not resolved.is_dir():
        raise ValueError(f"source is not a directory: {resolved}")
    manifests = list(resolved.glob("*.bdl.yaml"))
    if len(manifests) != 1:
        raise ValueError(
            f"expected exactly one *.bdl.yaml in {resolved}, found {len(manifests)}"
        )
    return resolved


def infer_coordinates(source: Path, args: argparse.Namespace) -> tuple[str, str, str]:
    parts = source.parts
    customer_type = args.customer_type
    bundle_name = args.bundle_name
    version = args.version

    if not (customer_type and bundle_name and version):
        if len(parts) >= 3:
            version = version or parts[-1]
            bundle_name = bundle_name or parts[-2]
            customer_type = customer_type or parts[-3]

    if not (customer_type and bundle_name and version):
        raise ValueError(
            "could not infer customer type, bundle name, and version; pass overrides"
        )

    return customer_type, bundle_name, version


def copy_tree(source: Path, destination: Path, force: bool) -> None:
    if destination.exists():
        if not force:
            raise ValueError(f"destination already exists: {destination}")
        shutil.rmtree(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(
        source,
        destination,
        ignore=shutil.ignore_patterns(".DS_Store", "__pycache__"),
    )


def vendor_known_extends(destination: Path, cac_tools_root: Path) -> set[str]:
    copied: set[str] = set()
    text_files = [
        path
        for path in destination.rglob("*")
        if path.is_file() and path.suffix in {".yaml", ".yml"}
    ]

    for path in text_files:
        original = path.read_text(encoding="utf-8")
        updated = original

        for external_path, spec in VENDORED_EXTENDS.items():
            pattern = re.compile(
                r"(?P<prefix>__extend__:\s*)"
                r"(?P<path>(?:\.\./)+"
                + re.escape(external_path)
                + r")"
            )
            if not pattern.search(updated):
                continue

            updated = pattern.sub(
                lambda match, replacement=spec["replacement"]: (
                    match.group("prefix") + replacement
                ),
                updated,
            )
            for source_rel, dest_rel in spec["copies"]:
                source_file = cac_tools_root / source_rel
                dest_file = destination / dest_rel
                if not source_file.is_file():
                    raise ValueError(f"required vendored source is missing: {source_file}")
                dest_file.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source_file, dest_file)
                copied.add(dest_rel)

        if updated != original:
            path.write_text(updated, encoding="utf-8")

    return copied


def find_external_extends(destination: Path) -> list[str]:
    problems: list[str] = []
    pattern = re.compile(r"__extend__:\s*(?P<path>(?:\.\./)+[^\s#]+)")
    for path in destination.rglob("*"):
        if not path.is_file() or path.suffix not in {".yaml", ".yml"}:
            continue
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            match = pattern.search(line)
            if match:
                rel = path.relative_to(destination)
                problems.append(f"{rel}:{line_no}: {match.group('path')}")
    return problems


def validate_output(repo_root: Path, destination: Path, version: str) -> None:
    sys.path.insert(0, str(repo_root / "scripts"))
    try:
        from converters.validator import BundleValidator
    except ModuleNotFoundError as exc:
        raise ValueError(
            "validation requires script dependencies; run with `uv run --with pyyaml`"
        ) from exc

    ok, errors = BundleValidator(verbose=True).validate_output(
        destination, expected_version=version
    )
    if not ok:
        raise ValueError("portable validation failed:\n" + "\n".join(errors))


def main() -> int:
    args = parse_args()
    try:
        source = resolve_source(args.source)
        repo_root = args.repo_root.expanduser().resolve()
        cac_tools_root = args.cac_tools_root.expanduser().resolve()
        customer_type, bundle_name, version = infer_coordinates(source, args)
        destination = repo_root / "portables" / customer_type / bundle_name / version

        copy_tree(source, destination, args.force)
        copied = vendor_known_extends(destination, cac_tools_root)
        external_extends = find_external_extends(destination)
        if external_extends:
            details = "\n".join(f"  - {entry}" for entry in external_extends)
            raise ValueError(f"unhandled external __extend__ references remain:\n{details}")

        if not args.skip_validation:
            validate_output(repo_root, destination, version)

        print(f"Imported portable bundle: {destination}")
        if copied:
            print("Vendored defaults:")
            for rel in sorted(copied):
                print(f"  - {rel}")
        return 0
    except ValueError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
