"""Validation utilities for bundle conversion."""

from pathlib import Path
from typing import List, Tuple

from utils.models import BundleAssets, BundleMetadata
from utils.file_utils import is_valid_json


class ValidationError(Exception):
    """Raised when validation fails."""
    pass


class BundleValidator:
    """Validates bundle inputs and outputs."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def validate_input(self, source_path: Path, assets: BundleAssets) -> Tuple[bool, List[str]]:
        """Validate input bundle structure and assets.

        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []

        # Check source path exists
        if not source_path.exists():
            errors.append(f"Source path does not exist: {source_path}")
            return False, errors

        if not source_path.is_dir():
            errors.append(f"Source path is not a directory: {source_path}")
            return False, errors

        # Check for at least some assets
        has_assets = (
            (assets.transforms_folder and assets.transforms) or
            (assets.dashboards_folder and assets.dashboards) or
            (assets.summaries_folder and assets.summaries)
        )

        if not has_assets:
            errors.append(
                "No valid assets found. Expected one of:\n"
                "  - transforms/ or transformations/ folder with JSON files\n"
                "  - dashboards/ or grafana/ folder with JSON files\n"
                "  - summaries/ folder with SQL files"
            )

        # Validate transform JSON files
        if assets.transforms:
            for transform in assets.transforms:
                if not transform.file_path.exists():
                    errors.append(f"Transform file not found: {transform.file_path}")
                elif not is_valid_json(transform.file_path):
                    errors.append(f"Invalid JSON in transform: {transform.file_path}")

        # Validate dashboard JSON files
        if assets.dashboards:
            for dashboard in assets.dashboards:
                if not dashboard.file_path.exists():
                    errors.append(f"Dashboard file not found: {dashboard.file_path}")
                elif not is_valid_json(dashboard.file_path):
                    errors.append(f"Invalid JSON in dashboard: {dashboard.file_path}")

        # Validate summary SQL files
        if assets.summaries:
            for summary in assets.summaries:
                if not summary.sql_file_path.exists():
                    errors.append(f"Summary SQL file not found: {summary.sql_file_path}")

        if self.verbose and not errors:
            print("✓ Input validation passed")

        return len(errors) == 0, errors

    def validate_metadata(self, metadata: BundleMetadata) -> Tuple[bool, List[str]]:
        """Validate bundle metadata.

        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []

        # Required fields
        if not metadata.customer_type:
            errors.append("customer_type is required")
        if not metadata.bundle_name:
            errors.append("bundle_name is required")
        if not metadata.version:
            errors.append("version is required")
        if not metadata.description:
            errors.append("description is required")
        if not metadata.maintainer:
            errors.append("maintainer is required")

        # Version format (basic semantic version check - allow X.Y or X.Y.Z)
        if metadata.version:
            parts = metadata.version.split('.')
            if len(parts) not in [2, 3] or not all(p.isdigit() for p in parts):
                errors.append(f"version must be semantic version (e.g., 1.0 or 1.0.0), got: {metadata.version}")

        if self.verbose and not errors:
            print("✓ Metadata validation passed")

        return len(errors) == 0, errors

    def validate_output(self, output_path: Path) -> Tuple[bool, List[str]]:
        """Validate generated bundle structure.

        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []

        if not output_path.exists():
            errors.append(f"Output path does not exist: {output_path}")
            return False, errors

        # Check for manifest file
        manifest_files = list(output_path.glob("*.bdl.yaml"))
        if not manifest_files:
            errors.append("No .bdl.yaml manifest file found")
        elif len(manifest_files) > 1:
            errors.append(f"Multiple .bdl.yaml files found: {[f.name for f in manifest_files]}")

        # Check for hydrolix directory and resources file
        hydrolix_dir = output_path / "hydrolix"
        if hydrolix_dir.exists():
            resources_file = hydrolix_dir / "resources.hdp.yaml"
            if not resources_file.exists():
                errors.append("hydrolix/resources.hdp.yaml not found")

        # Check for grafana directory and resources file (if dashboards were present)
        grafana_dir = output_path / "grafana"
        if grafana_dir.exists():
            resources_file = grafana_dir / "resources.gfo.yaml"
            if not resources_file.exists():
                errors.append("grafana/resources.gfo.yaml not found")

        if self.verbose and not errors:
            print("✓ Output validation passed")

        return len(errors) == 0, errors
