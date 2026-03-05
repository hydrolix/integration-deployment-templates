"""Hydrolix resource generator."""

from pathlib import Path
from typing import List

from utils.models import BundleAssets, Transform
from utils.yaml_utils import dump_yaml
import json

from utils.file_utils import write_file, sanitize_cac_name


class HydrolixGenerator:
    """Generates Hydrolix resources (tables, transforms, resources.hdp.yaml)."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def generate(self, output_path: Path, assets: BundleAssets, table_name: str = "logs", metadata=None):
        """Generate all Hydrolix resources.

        Args:
            output_path: Path to bundle output directory
            assets: Discovered bundle assets
            table_name: Name for the table (default: logs)
            metadata: Optional BundleMetadata for description etc.
        """
        hydrolix_dir = output_path / "hydrolix"

        # Generate table definition and transforms
        if assets.transforms:
            self._generate_table(hydrolix_dir, table_name, metadata=metadata)
            self._copy_transforms(hydrolix_dir, assets.transforms)

        # Generate main resources file
        self._generate_resources_file(hydrolix_dir, assets, table_name)

        if self.verbose:
            print(f"✓ Generated Hydrolix resources")

    def _generate_table(self, hydrolix_dir: Path, table_name: str, metadata=None):
        """Generate table YAML file."""
        tables_dir = hydrolix_dir / "tables"
        tables_dir.mkdir(parents=True, exist_ok=True)

        description = metadata.description if metadata else 'Integration logs'

        # Create table YAML with reference to base defaults
        table_yaml = {
            '__extend__': '../../../../../../hydrolix/_defaults/table_defaults.yaml',
            'description': description,
            'type': 'turbine',
        }

        table_filename = f"table_{table_name}.yaml"
        table_path = tables_dir / table_filename
        write_file(table_path, dump_yaml(table_yaml, sort_keys=False))

        if self.verbose:
            print(f"  Generated table: {table_filename}")

    def _copy_transforms(self, hydrolix_dir: Path, transforms: List[Transform]):
        """Copy transform JSON files, extracting sql_transform and sample_data into separate files."""
        transforms_dir = hydrolix_dir / "transforms"
        transforms_dir.mkdir(parents=True, exist_ok=True)

        strip_fields = {'created', 'modified', 'table', 'url', 'uuid'}

        for transform in transforms:
            # Read, strip metadata
            with open(transform.file_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            for field in strip_fields:
                data.pop(field, None)

            sanitized_name = sanitize_cac_name(transform.name)
            settings = data.get('settings', {})

            # Extract sql_transform to separate .sql file
            sql_transform = settings.pop('sql_transform', None)
            if sql_transform:
                sql_filename = f"{sanitized_name}_select.sql"
                sql_path = transforms_dir / sql_filename
                write_file(sql_path, sql_transform)
                if self.verbose:
                    print(f"  Extracted SQL: {sql_filename}")

            # Extract sample_data to separate .json file
            sample_data = settings.pop('sample_data', None)
            if sample_data is not None:
                sample_filename = f"{sanitized_name}_sample_data.json"
                sample_path = transforms_dir / sample_filename
                with open(sample_path, 'w', encoding='utf-8') as f:
                    json.dump(sample_data, f, indent=2, ensure_ascii=False)
                    f.write('\n')
                if self.verbose:
                    print(f"  Extracted sample data: {sample_filename}")

            # Write cleaned transform JSON (without sql_transform and sample_data)
            transform_filename = f"{sanitized_name}.json"
            transform_dest = transforms_dir / transform_filename
            with open(transform_dest, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
                f.write('\n')

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
                sanitized_name = sanitize_cac_name(transform.name)
                transform_entry = {
                    '__extend__': f'transforms/{sanitized_name}.json',
                    'settings': {
                        'sql_transform': {
                            '__extend__': f'transforms/{sanitized_name}_select.sql'
                        },
                        'sample_data': {
                            '__extend__': f'transforms/{sanitized_name}_sample_data.json'
                        }
                    }
                }
                transforms_dict[sanitized_name] = transform_entry

            resources['transforms'] = {
                table_name: transforms_dict
            }

        # Write resources file
        hydrolix_dir.mkdir(parents=True, exist_ok=True)
        resources_path = hydrolix_dir / "resources.hdp.yaml"
        write_file(resources_path, dump_yaml(resources, sort_keys=False))

        if self.verbose:
            print(f"  Generated resources.hdp.yaml")
