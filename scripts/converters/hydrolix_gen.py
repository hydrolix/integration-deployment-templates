"""Hydrolix resource generator."""

from pathlib import Path
from typing import List

from utils.models import BundleAssets, Transform
from utils.yaml_utils import dump_yaml
from utils.file_utils import write_file, copy_file


class HydrolixGenerator:
    """Generates Hydrolix resources (tables, transforms, resources.hdp.yaml)."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def generate(self, output_path: Path, assets: BundleAssets, table_name: str = "logs"):
        """Generate all Hydrolix resources.

        Args:
            output_path: Path to bundle output directory
            assets: Discovered bundle assets
            table_name: Name for the table (default: logs)
        """
        hydrolix_dir = output_path / "hydrolix"

        # Generate table definition and transforms
        if assets.transforms:
            self._generate_table(hydrolix_dir, table_name)
            self._copy_transforms(hydrolix_dir, assets.transforms)

        # Generate main resources file
        self._generate_resources_file(hydrolix_dir, assets, table_name)

        if self.verbose:
            print(f"✓ Generated Hydrolix resources")

    def _generate_table(self, hydrolix_dir: Path, table_name: str):
        """Generate table YAML file."""
        tables_dir = hydrolix_dir / "tables"
        tables_dir.mkdir(parents=True, exist_ok=True)

        # Create table YAML with reference to base defaults
        table_yaml = {
            '__extend__': '../../../../../../hydrolix/_defaults/table_defaults.yaml',
            'description': 'Akamai Observability Platform logs',
            'type': 'turbine',
            'settings': {
                'stream': {
                    'cold_data_max_age_days': 365,
                    'token_auth_enabled': True
                }
            }
        }

        table_filename = f"table_{table_name}.yaml"
        table_path = tables_dir / table_filename
        write_file(table_path, dump_yaml(table_yaml, sort_keys=False))

        if self.verbose:
            print(f"  Generated table: {table_filename}")

    def _copy_transforms(self, hydrolix_dir: Path, transforms: List[Transform]):
        """Copy transform JSON files."""
        transforms_dir = hydrolix_dir / "transforms"
        transforms_dir.mkdir(parents=True, exist_ok=True)

        for transform in transforms:
            # Use the transform name from the JSON file for the output filename
            transform_filename = f"{transform.name}.json"
            transform_dest = transforms_dir / transform_filename
            copy_file(transform.file_path, transform_dest)

            if self.verbose:
                print(f"  Copied transform: {transform_filename}")

    def _generate_resources_file(self, hydrolix_dir: Path, assets: BundleAssets, table_name: str):
        """Generate main resources.hdp.yaml file with nested structure."""
        resources = {}

        # Add tables section with nested structure
        if assets.transforms:
            resources['tables'] = {
                table_name: {
                    '__extend__': f'tables/table_{table_name}.yaml'
                }
            }

        # Add transforms section with nested structure
        if assets.transforms:
            transforms_dict = {}
            for transform in assets.transforms:
                transforms_dict[transform.name] = {
                    '__extend__': f'transforms/{transform.name}.json'
                }

            resources['transforms'] = {
                table_name: transforms_dict
            }

        # Write resources file
        resources_path = hydrolix_dir / "resources.hdp.yaml"
        write_file(resources_path, dump_yaml(resources, sort_keys=False))

        if self.verbose:
            print(f"  Generated resources.hdp.yaml")
