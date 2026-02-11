"""Shared file utilities for bundle scripts."""

import json
import os


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
