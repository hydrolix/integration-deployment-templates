"""Validation utilities for bundle conversion."""
import json
from pathlib import Path
from typing import List, Optional, Tuple

import yaml

from utils.models import BundleAssets, BundleMetadata
from utils.file_utils import is_valid_json


class ValidationError(Exception):
    """Raised when validation fails."""
    pass


class BundleValidator:
    """Validates bundle inputs and outputs."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    # checks rouce path, asset presence, valid JSON
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

    # checks required fields, format
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

    # checks version consistency across bundle-config.json, bundle.json, output path, and .bdl.yaml
    def validate_version_consistency(
        self,
        source_path: Path,
        expected_version: str,
        bundle_config_path: Optional[Path] = None,
    ) -> Tuple[bool, List[str]]:
        """Validate that version is consistent across bundle-config.json, bundle.json,
        the portables output path, and the .bdl.yaml manifest.

        Args:
            source_path: Source bundle directory (may contain bundle.json and bundle-config.json)
            expected_version: The version passed on the CLI / from metadata
            bundle_config_path: Explicit path to a bundle-config.json; if None, looks for
                                 bundle-config.json inside source_path

        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []

        # Resolve bundle-config.json path
        config_path = bundle_config_path or (source_path / "bundle-config.json")

        # Load bundle.json version (may not exist yet at stage-1 time)
        bundle_json_path = source_path / "bundle.json"
        bundle_json_version: Optional[str] = None
        if bundle_json_path.exists():
            try:
                with open(bundle_json_path, "r") as f:
                    bundle_data = json.load(f)
                bundle_json_version = bundle_data.get("metadata", {}).get("version", "")
            except json.JSONDecodeError:
                errors.append(f"Invalid JSON in bundle.json: {bundle_json_path}")

        if bundle_json_version and bundle_json_version != expected_version:
            errors.append(
                f"Version mismatch: bundle.json metadata.version '{bundle_json_version}' "
                f"does not match expected version '{expected_version}'"
            )

        # Load bundle-config.json version if it exists
        if config_path.exists():
            try:
                with open(config_path, "r") as f:
                    config_data = json.load(f)
                config_version = config_data.get("version", "")
                if config_version:
                    if config_version != expected_version:
                        errors.append(
                            f"Version mismatch: {config_path.name} version '{config_version}' "
                            f"does not match expected version '{expected_version}'"
                        )
                    if bundle_json_version and config_version != bundle_json_version:
                        errors.append(
                            f"Version mismatch: {config_path.name} version '{config_version}' "
                            f"does not match bundle.json metadata.version '{bundle_json_version}'"
                        )
            except json.JSONDecodeError:
                errors.append(f"Invalid JSON in {config_path.name}: {config_path}")

        if self.verbose and not errors:
            print("✓ Version consistency validation passed")

        return len(errors) == 0, errors

    # checks output path, manifest files
    def validate_output(self, output_path: Path, expected_version: Optional[str] = None) -> Tuple[bool, List[str]]:
        """Validate generated bundle structure.

        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []

        if not output_path.exists():
            errors.append(f"Output path does not exist: {output_path}")
            return False, errors

        # Check output path version segment matches expected version
        if expected_version:
            path_version = output_path.name
            if path_version != expected_version:
                errors.append(
                    f"Version mismatch: portables output directory '{path_version}' "
                    f"does not match expected version '{expected_version}'"
                )

        # Check for manifest file
        manifest_files = list(output_path.glob("*.bdl.yaml"))
        if not manifest_files:
            errors.append("No .bdl.yaml manifest file found")
        elif len(manifest_files) > 1:
            errors.append(f"Multiple .bdl.yaml files found: {[f.name for f in manifest_files]}")
        elif expected_version:
            # Check .bdl.yaml version field matches expected version
            manifest_path = manifest_files[0]
            try:
                with open(manifest_path, "r") as f:
                    manifest_data = yaml.safe_load(f) or {}
                manifest_version = manifest_data.get("version", "")
                if manifest_version and manifest_version != expected_version:
                    errors.append(
                        f"Version mismatch: {manifest_path.name} version '{manifest_version}' "
                        f"does not match expected version '{expected_version}'"
                    )
            except yaml.YAMLError as exc:
                errors.append(f"Invalid YAML in {manifest_path.name}: {exc}")

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

        # check for format of dashboard JSON files for Grafana
        dashboard_dir = output_path / "grafana" / "dashboards"
        if dashboard_dir.exists():
            # Load resources.gfo.yaml inputs for cross-checking
            gfo_datasource_keys = set()
            resources_file = output_path / "grafana" / "resources.gfo.yaml"
            if resources_file.exists():
                with open(resources_file, 'r') as f:
                    gfo = yaml.safe_load(f) or {}
                for dash_entry in (gfo.get('dashboards') or {}).values():
                    for key in (dash_entry.get('inputs') or {}):
                        gfo_datasource_keys.add(key)

            for dashboard in dashboard_dir.glob("*.json"):
                with open(dashboard, 'r') as f:
                    try:
                        json_file = json.load(f)
                    except json.JSONDecodeError:
                        errors.append(f"Invalid JSON format in dashboard: {dashboard}")
                        continue
                    # check for __inputs section
                    if '__inputs' not in json_file:
                        errors.append(f"Dashboard JSON missing __inputs section: {dashboard}")
                        continue
                    # check datasource input names match expected value
                    expected_ds_name = 'DS_HYDROLIX-HYDROLIX-DATASOURCE'
                    for inp in json_file['__inputs']:
                        if inp.get('type') == 'datasource':
                            actual_name = inp.get('name')
                            if actual_name != expected_ds_name:
                                errors.append(
                                    f"Dashboard {dashboard.name}: datasource input name "
                                    f"'{actual_name}' does not match expected '{expected_ds_name}'"
                                )
                            # cross-check against resources.gfo.yaml inputs keys
                            if gfo_datasource_keys and actual_name not in gfo_datasource_keys:
                                errors.append(
                                    f"Dashboard {dashboard.name}: datasource input name "
                                    f"'{actual_name}' not found in resources.gfo.yaml inputs "
                                    f"(found: {sorted(gfo_datasource_keys)})"
                                )

        if self.verbose and not errors:
            print("✓ Output validation passed")

        return len(errors) == 0, errors
