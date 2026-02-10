"""File operation utilities."""

import json
import shutil
from pathlib import Path
from typing import Any, Dict


def is_valid_json(file_path: Path) -> bool:
    """Check if a file contains valid JSON."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            json.load(f)
        return True
    except (json.JSONDecodeError, FileNotFoundError):
        return False


def read_json(file_path: Path) -> Dict[str, Any]:
    """Read and parse JSON file."""
    with open(file_path, 'r', encoding='utf-8') as f:
        return json.load(f)


def copy_file(src: Path, dst: Path):
    """Copy a file, creating parent directories if needed."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)


def write_file(file_path: Path, content: str):
    """Write content to file, creating parent directories if needed."""
    file_path.parent.mkdir(parents=True, exist_ok=True)
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)


def sanitize_filename(name: str) -> str:
    """Sanitize a name for use as a filename or identifier."""
    # Replace spaces and special chars with hyphens
    sanitized = name.lower().replace(' ', '-')
    # Remove any remaining special characters except hyphens and underscores
    sanitized = ''.join(c for c in sanitized if c.isalnum() or c in '-_')
    return sanitized
