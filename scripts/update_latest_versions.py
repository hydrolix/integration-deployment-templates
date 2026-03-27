#!/usr/bin/env python3
"""Update data/bundles/latest_versions.yaml with the latest semver version for each bundle.

Walks a bundles directory, finds every bundle root (a directory whose children
include at least one semver-versioned subdirectory), picks the highest version,
and writes the result to latest_versions.yaml.

Usage:
    python3 scripts/update_latest_versions.py <bundles_dir>

Example:
    python3 scripts/update_latest_versions.py cac-tools/data/bundles
"""

import os
import re
import sys

SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")


def _semver_key(v):
    return tuple(int(x) for x in v.split("."))


def find_bundle_roots(bundles_dir):
    """Yield (bundle_rel_path, latest_version, bdl_yaml_filename) for each bundle root."""
    for dirpath, dirnames, _ in os.walk(bundles_dir):
        rel = os.path.relpath(dirpath, bundles_dir)
        if rel == ".":
            continue

        # Don't descend into version directories
        if SEMVER_RE.match(os.path.basename(dirpath)):
            dirnames.clear()
            continue

        versions = sorted(
            [d for d in dirnames if SEMVER_RE.match(d)],
            key=_semver_key,
        )
        if not versions:
            continue

        latest = versions[-1]
        version_dir = os.path.join(dirpath, latest)
        bdl_files = [f for f in os.listdir(version_dir) if f.endswith(".bdl.yaml")]
        if not bdl_files:
            continue

        bundle_rel = rel.replace(os.sep, "/")
        yield bundle_rel, latest, bdl_files[0]


def load_latest_versions(path):
    """Load latest_versions.yaml, returning (header_lines, entries_dict)."""
    header_lines = []
    entries = {}
    if not os.path.exists(path):
        return header_lines, entries
    with open(path) as f:
        for line in f:
            stripped = line.rstrip()
            if stripped.startswith("#"):
                header_lines.append(stripped)
            elif ":" in stripped:
                k, v = stripped.split(":", 1)
                entries[k.strip()] = v.strip()
    return header_lines, entries


def write_latest_versions(path, header_lines, entries):
    """Write latest_versions.yaml, preserving header comments."""
    with open(path, "w") as f:
        for line in header_lines:
            f.write(line + "\n")
        for k, v in sorted(entries.items()):
            f.write(f"{k}: {v}\n")


def main():
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <bundles_dir>", file=sys.stderr)
        sys.exit(1)

    bundles_dir = sys.argv[1]
    latest_versions_file = os.path.join(bundles_dir, "latest_versions.yaml")

    if not os.path.isdir(bundles_dir):
        print(f"Error: bundles directory not found: {bundles_dir}", file=sys.stderr)
        sys.exit(1)

    header_lines, entries = load_latest_versions(latest_versions_file)

    # Only the latest_versions.yaml pointer file is written — version directories
    # are never modified, deleted, or overwritten by this script.
    changed = False
    for bundle_rel, latest, bdl_filename in find_bundle_roots(bundles_dir):
        key = re.sub(r"[-/]", "_", bundle_rel)
        value = f"{bundle_rel}/{latest}/{bdl_filename}"
        if entries.get(key) != value:
            print(f"  {'updating' if key in entries else 'adding'} {key}: {value}")
            entries[key] = value
            changed = True

    if changed:
        write_latest_versions(latest_versions_file, header_lines, entries)
        print(f"✓ Updated {latest_versions_file}")
    else:
        print("✓ latest_versions.yaml already up to date")


if __name__ == "__main__":
    main()
