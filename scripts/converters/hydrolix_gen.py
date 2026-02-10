"""Hydrolix resource generator."""

from pathlib import Path
from typing import List

from utils.models import BundleAssets, Transform, Summary
from utils.yaml_utils import dump_yaml
from utils.file_utils import write_file, copy_file


class HydrolixGenerator:
    """Generates Hydrolix resources (tables, summaries, resources.hdp.yaml)."""

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

        # Generate table definitions
        if assets.transforms:
            self._generate_tables(hydrolix_dir, assets.transforms, table_name)

        # Generate summary definitions
        if assets.summaries:
            self._generate_summaries(hydrolix_dir, assets.summaries)

        # Generate main resources file
        self._generate_resources_file(hydrolix_dir, assets, table_name)

        if self.verbose:
            print(f"✓ Generated Hydrolix resources")

    def _generate_tables(self, hydrolix_dir: Path, transforms: List[Transform], table_name: str):
        """Generate table YAML files."""
        tables_dir = hydrolix_dir / "tables"
        tables_dir.mkdir(parents=True, exist_ok=True)

        for transform in transforms:
            # Copy transform JSON to transforms/ subdirectory
            transforms_dir = hydrolix_dir / "transforms"
            transforms_dir.mkdir(parents=True, exist_ok=True)

            transform_filename = f"{transform.name}.json"
            transform_dest = transforms_dir / transform_filename
            copy_file(transform.file_path, transform_dest)

            if self.verbose:
                print(f"  Copied transform: {transform_filename}")

            # Create table YAML with __extend__ reference
            table_yaml = {
                'name': table_name,
                'description': f'Table for {transform.name}',
                '__extend__': f'../transforms/{transform_filename}'
            }

            table_filename = f"{table_name}.hdx.yaml"
            table_path = tables_dir / table_filename
            write_file(table_path, dump_yaml(table_yaml, sort_keys=False))

            if self.verbose:
                print(f"  Generated table: {table_filename}")

    def _generate_summaries(self, hydrolix_dir: Path, summaries: List[Summary]):
        """Generate summary YAML files."""
        summaries_dir = hydrolix_dir / "summaries"
        summaries_dir.mkdir(parents=True, exist_ok=True)

        # Copy SQL files to sql/ subdirectory
        sql_dir = hydrolix_dir / "sql"
        sql_dir.mkdir(parents=True, exist_ok=True)

        for summary in summaries:
            # Copy SQL file
            sql_filename = f"{summary.name}.sql"
            sql_dest = sql_dir / sql_filename
            copy_file(summary.sql_file_path, sql_dest)

            if self.verbose:
                print(f"  Copied SQL: {sql_filename}")

            # Create summary YAML with __extend__ reference
            summary_yaml = {
                'name': summary.name,
                'description': f'Summary table: {summary.name}',
                '__extend__': f'../sql/{sql_filename}'
            }

            summary_filename = f"{summary.name}.hdx.yaml"
            summary_path = summaries_dir / summary_filename
            write_file(summary_path, dump_yaml(summary_yaml, sort_keys=False))

            if self.verbose:
                print(f"  Generated summary: {summary_filename}")

    def _generate_resources_file(self, hydrolix_dir: Path, assets: BundleAssets, table_name: str):
        """Generate main resources.hdp.yaml file."""
        resources = {}

        # Add tables section if transforms exist
        if assets.transforms:
            resources['tables'] = [f'tables/{table_name}.hdx.yaml']

        # Add summaries section if summaries exist
        if assets.summaries:
            summary_refs = [f'summaries/{s.name}.hdx.yaml' for s in assets.summaries]
            resources['summaries'] = summary_refs

        # Write resources file
        resources_path = hydrolix_dir / "resources.hdp.yaml"
        write_file(resources_path, dump_yaml(resources, sort_keys=False))

        if self.verbose:
            print(f"  Generated resources.hdp.yaml")
