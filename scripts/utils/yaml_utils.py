"""YAML formatting utilities."""

import yaml
from typing import Any, Dict


def dump_yaml(data: Dict[str, Any], sort_keys: bool = False) -> str:
    """
    Dump data to YAML string with consistent formatting.

    Args:
        data: Dictionary to convert to YAML
        sort_keys: Whether to sort dictionary keys

    Returns:
        Formatted YAML string
    """
    return yaml.dump(
        data,
        default_flow_style=False,
        sort_keys=sort_keys,
        allow_unicode=True,
        width=120
    )


def format_yaml_with_comments(base_yaml: str, comments: Dict[str, str]) -> str:
    """
    Add comments to YAML output.

    Args:
        base_yaml: Base YAML string
        comments: Dict mapping keys to comment text

    Returns:
        YAML with comments added
    """
    lines = base_yaml.split('\n')
    result = []

    for line in lines:
        result.append(line)
        # Add comments after specific keys
        for key, comment in comments.items():
            if line.strip().startswith(f'{key}:'):
                result.append(f'  # {comment}')

    return '\n'.join(result)
