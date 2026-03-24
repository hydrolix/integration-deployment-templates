"""Manages the .originals/ directory that preserves raw bundle assets for clean pipeline re-runs."""

import os
import shutil


def backup_to_originals(bundle_dir: str, repo_root: str) -> str:
    """Back up raw assets to .originals/<relative_bundle_path>/.

    Excludes bundle.json and bundle-config.json. Returns the originals path.
    """
    EXCLUDE_FILES = {"bundle.json", "bundle-config.json"}

    rel_path = os.path.relpath(bundle_dir, repo_root)
    originals_path = os.path.join(repo_root, ".originals", rel_path)

    if os.path.exists(originals_path):
        shutil.rmtree(originals_path)

    shutil.copytree(
        bundle_dir,
        originals_path,
        ignore=shutil.ignore_patterns(*EXCLUDE_FILES),
    )

    return originals_path


def restore_from_originals(bundle_dir: str, repo_root: str) -> None:
    """Restore raw assets from .originals/ into bundle_dir.

    Deletes everything in bundle_dir except bundle-config.json, then copies originals back.
    """
    PRESERVE_FILES = {"bundle-config.json"}

    rel_path = os.path.relpath(bundle_dir, repo_root)
    originals_path = os.path.join(repo_root, ".originals", rel_path)

    # Delete everything in bundle_dir except preserved files
    for entry in os.listdir(bundle_dir):
        if entry in PRESERVE_FILES:
            continue
        entry_path = os.path.join(bundle_dir, entry)
        if os.path.isdir(entry_path):
            shutil.rmtree(entry_path)
        else:
            os.remove(entry_path)

    # Copy originals back into bundle_dir
    for entry in os.listdir(originals_path):
        src = os.path.join(originals_path, entry)
        dst = os.path.join(bundle_dir, entry)
        if os.path.isdir(src):
            shutil.copytree(src, dst)
        else:
            shutil.copy2(src, dst)


def update_originals(bundle_dir: str, repo_root: str) -> str:
    """Delete existing .originals/ for this bundle, re-backup from current bundle_dir content.

    Returns the originals path.
    """
    rel_path = os.path.relpath(bundle_dir, repo_root)
    originals_path = os.path.join(repo_root, ".originals", rel_path)

    if os.path.exists(originals_path):
        shutil.rmtree(originals_path)

    return backup_to_originals(bundle_dir, repo_root)
