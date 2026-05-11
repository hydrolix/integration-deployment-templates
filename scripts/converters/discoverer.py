"""Asset discovery module."""

import json
from pathlib import Path
from typing import List, Optional

from utils.models import BundleAssets, Transform, Dashboard, DashboardInput, Summary, Function, Dictionary
from utils.file_utils import read_json, sanitize_filename


class AssetDiscoverer:
    """Discovers and catalogs bundle assets."""

    def __init__(self, source_path: Path, verbose: bool = False):
        self.source_path = Path(source_path)
        self.verbose = verbose

    def discover(self) -> BundleAssets:
        """Scan directory and return structured asset catalog."""
        assets = BundleAssets()

        # Find asset folders
        assets.transforms_folder = self._find_transforms_folder()
        assets.dashboards_folder = self._find_dashboards_folder()
        assets.summaries_folder = self._find_summaries_folder()

        if self.verbose:
            print(f"Found transforms folder: {assets.transforms_folder}")
            print(f"Found dashboards folder: {assets.dashboards_folder}")
            print(f"Found summaries folder: {assets.summaries_folder}")

        # Discover assets
        if assets.transforms_folder:
            assets.transforms = self._discover_transforms(assets.transforms_folder)
        if assets.dashboards_folder:
            assets.dashboards = self._discover_dashboards(assets.dashboards_folder)
        if assets.summaries_folder:
            assets.summaries = self._discover_summaries(assets.summaries_folder)

        assets.functions_folder = self._find_functions_folder()
        if assets.functions_folder:
            assets.functions = self._discover_functions(assets.functions_folder)
        assets.dictionaries_folder = self._find_dictionaries_folder()
        if assets.dictionaries_folder:
            assets.dictionaries = self._discover_dictionaries(assets.dictionaries_folder)

        return assets

    def _find_transforms_folder(self) -> Optional[Path]:
        """Find the transforms/transformations folder."""
        # Check for transformations/ first (preferred)
        transforms_path = self.source_path / 'transformations'
        if transforms_path.exists() and transforms_path.is_dir():
            return transforms_path

        # Check for transforms/ (alternate)
        transforms_path = self.source_path / 'transforms'
        if transforms_path.exists() and transforms_path.is_dir():
            return transforms_path

        return None

    def _find_dashboards_folder(self) -> Optional[Path]:
        """Find the dashboards/grafana folder."""
        # Check for dashboards/ first
        dashboards_path = self.source_path / 'dashboards'
        if dashboards_path.exists() and dashboards_path.is_dir():
            return dashboards_path

        # Check for grafana/
        grafana_path = self.source_path / 'grafana'
        if grafana_path.exists() and grafana_path.is_dir():
            return grafana_path

        return None

    def _find_summaries_folder(self) -> Optional[Path]:
        """Find the summaries folder."""
        summaries_path = self.source_path / 'summaries'
        if summaries_path.exists() and summaries_path.is_dir():
            return summaries_path
        return None

    def _discover_transforms(self, folder: Path) -> List[Transform]:
        """Find and parse transform files."""
        transforms = []

        # Check for single transform.json
        single_transform = folder / 'transform.json'
        if single_transform.exists():
            transform_data = read_json(single_transform)
            name = transform_data.get('name', 'default_transform')
            transforms.append(Transform(
                name=name,
                file_path=single_transform,
                table_name='logs'  # Default, can be overridden
            ))
            if self.verbose:
                print(f"  Found single transform: {name}")
            return transforms

        # Check for multiple transform files in subdirectories
        for item in folder.iterdir():
            if item.is_dir():
                transform_file = item / 'transform.json'
                if transform_file.exists():
                    transform_data = read_json(transform_file)
                    name = transform_data.get('name', item.name)
                    transforms.append(Transform(
                        name=name,
                        file_path=transform_file,
                        table_name='logs'
                    ))
                    if self.verbose:
                        print(f"  Found transform: {name} in {item.name}/")

        # Also check for transform JSON files directly in the folder
        for item in folder.iterdir():
            if item.is_file() and item.suffix == '.json' and item.name not in ['sample_data.json', 'sample_data_template.json']:
                try:
                    transform_data = read_json(item)
                    # Check if it's a transform file (has 'settings' with 'output_columns' or has 'type' field)
                    is_transform = False
                    if 'settings' in transform_data:
                        settings = transform_data['settings']
                        if isinstance(settings, dict) and 'output_columns' in settings:
                            is_transform = True
                    elif 'type' in transform_data and 'name' in transform_data:
                        is_transform = True

                    if is_transform:
                        name = transform_data.get('name', item.stem)
                        transforms.append(Transform(
                            name=name,
                            file_path=item,
                            table_name='logs'
                        ))
                        if self.verbose:
                            print(f"  Found transform: {name}")
                except Exception:
                    pass  # Skip invalid JSON files

        return transforms

    def _discover_dashboards(self, folder: Path) -> List[Dashboard]:
        """Find dashboard JSON files and extract metadata."""
        dashboards = []

        for json_file in folder.rglob('*.json'):
            try:
                dashboard_data = read_json(json_file)

                # Extract __inputs array
                inputs = []
                if '__inputs' in dashboard_data:
                    for inp in dashboard_data['__inputs']:
                        inputs.append(DashboardInput(
                            name=inp.get('name', ''),
                            type=inp.get('type', ''),
                            value=inp.get('value')
                        ))

                # Determine relative path and folder
                rel_path = json_file.relative_to(folder)
                folder_name = rel_path.parent.name if rel_path.parent != Path('.') else 'main'
                folder_uid = f"hdx-{sanitize_filename(folder_name)}-folder"

                dashboards.append(Dashboard(
                    filename=json_file.name,
                    file_path=json_file,
                    relative_path=str(rel_path),
                    folder_uid=folder_uid if folder_name != 'main' else 'hdx-main-folder',
                    inputs=inputs
                ))

                if self.verbose:
                    print(f"  Found dashboard: {json_file.name} ({len(inputs)} inputs)")

            except Exception as e:
                if self.verbose:
                    print(f"  Warning: Could not parse dashboard {json_file}: {e}")
                continue

        return dashboards

    def _discover_summaries(self, folder: Path) -> List[Summary]:
        """Find summary SQL files."""
        summaries = []

        for sql_file in sorted(folder.glob('*.sql')):
            name = sql_file.stem
            summaries.append(Summary(
                name=name,
                sql_file_path=sql_file
            ))
            if self.verbose:
                print(f"  Found summary: {name}")

        return summaries

    def _load_dep_names(self, key: str) -> set:
        """Return union of required_<key> and shared_<key> from bundle.json."""
        bundle_json = self.source_path / 'bundle.json'
        if not bundle_json.exists():
            return set()
        with open(bundle_json, 'r', encoding='utf-8') as f:
            bundle = json.load(f)
        deps = bundle.get('dependencies', {}).get('hydrolix', {})
        required = set(deps.get(f'required_{key}', []))
        shared = set(deps.get(f'shared_{key}', []))
        return required | shared

    def _find_functions_folder(self) -> Optional[Path]:
        """Find the functions folder."""
        path = self.source_path / 'functions'
        return path if path.exists() and path.is_dir() else None

    def _discover_functions(self, folder: Path) -> List[Function]:
        """Find function JSON files, filtered by names declared in bundle.json."""
        names = self._load_dep_names('functions')
        results = []
        for json_file in sorted(folder.glob('*.json')):
            if json_file.stem in names:
                results.append(Function(name=json_file.stem, file_path=json_file))
                if self.verbose:
                    print(f"  Found function: {json_file.name}")
        return results

    def _find_dictionaries_folder(self) -> Optional[Path]:
        """Find the dictionaries folder."""
        path = self.source_path / 'dictionaries'
        return path if path.exists() and path.is_dir() else None

    def _discover_dictionaries(self, folder: Path) -> List[Dictionary]:
        """Find dictionary schema files for names declared in bundle.json."""
        names = self._load_dep_names('dictionaries')
        if not names:
            names = {
                item.name
                for item in folder.iterdir()
                if item.is_dir() and (item / 'schema_definition.json').exists()
            }
        results = []
        for name in sorted(names):
            schema = folder / name / 'schema_definition.json'
            if schema.exists():
                results.append(Dictionary(name=name, file_path=schema))
                if self.verbose:
                    print(f"  Found dictionary: {name}")
        return results
