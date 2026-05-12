"""Shared file utilities for bundle scripts."""

import json
import os
import re
import shutil


def read_json(filepath):
    """Read and parse a JSON file."""
    with open(filepath, "r", encoding="utf-8") as f:
        return json.load(f)


def write_json(filepath, data, indent=2):
    """Write data to a JSON file with consistent formatting."""
    with open(filepath, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=indent, ensure_ascii=False)
        f.write("\n")


def write_file(filepath, content):
    """Write string content to a file."""
    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content)


def is_valid_json(filepath):
    """Check if a file contains valid JSON."""
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            json.load(f)
        return True
    except (json.JSONDecodeError, FileNotFoundError):
        return False


def ensure_dir(dirpath):
    """Create directory if it doesn't exist."""
    os.makedirs(dirpath, exist_ok=True)


def copy_file(src, dst):
    """Copy a file, creating parent directories as needed."""
    os.makedirs(os.path.dirname(str(dst)), exist_ok=True)
    shutil.copy2(str(src), str(dst))


def sanitize_cac_name(name):
    """Lowercase + underscores only for CAC file names and YAML keys."""
    sanitized = name.lower()
    sanitized = re.sub(r"[^a-z0-9_]", "_", sanitized)
    sanitized = re.sub(r"_{2,}", "_", sanitized)
    return sanitized.strip("_")


def sanitize_filename(name):
    """Sanitize a string for use as a filename or UID component.

    Replaces non-alphanumeric characters (except hyphens/underscores) with hyphens
    and collapses multiple hyphens.
    """
    sanitized = re.sub(r"[^a-zA-Z0-9_-]", "-", name)
    sanitized = re.sub(r"-{2,}", "-", sanitized)
    return sanitized.strip("-")


def slugify_grafana_title(title: str) -> str:
    """Convert a Grafana dashboard title to its URL slug.

    Mirrors Grafana's own slugification: lowercase, any run of non-alphanumeric
    ASCII characters collapses to a single hyphen, leading/trailing hyphens
    stripped. Non-ASCII characters (accents, etc.) are treated as separators.

    Examples:
        "Raw Logs"             -> "raw-logs"
        "CDN Dashboard Default"-> "cdn-dashboard-default"
        "Cache Analysis Treemap" -> "cache-analysis-treemap"
    """
    slug = title.lower()
    slug = re.sub(r"[^a-z0-9]+", "-", slug)
    return slug.strip("-")
