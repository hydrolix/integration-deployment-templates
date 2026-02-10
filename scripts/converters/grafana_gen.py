"""Grafana resource generator."""

from pathlib import Path
from typing import List, Dict, Set

from utils.models import BundleAssets, Dashboard
from utils.yaml_utils import dump_yaml
from utils.file_utils import write_file, copy_file


class GrafanaGenerator:
    """Generates Grafana resources (dashboards, folders, resources.gfo.yaml)."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def generate(self, output_path: Path, assets: BundleAssets, home_dashboard: str = None):
        """Generate all Grafana resources.

        Args:
            output_path: Path to bundle output directory
            assets: Discovered bundle assets
            home_dashboard: Optional home dashboard filename
        """
        grafana_dir = output_path / "grafana"

        if not assets.dashboards:
            if self.verbose:
                print("  No dashboards found, skipping Grafana generation")
            return

        # Copy dashboards
        dashboard_paths = self._copy_dashboards(grafana_dir, assets.dashboards)

        # Collect folder information
        folders = self._collect_folders(assets.dashboards)

        # Generate main resources file
        self._generate_resources_file(grafana_dir, assets.dashboards, folders, dashboard_paths, home_dashboard)

        if self.verbose:
            print(f"✓ Generated Grafana resources")

    def _copy_dashboards(self, grafana_dir: Path, dashboards: List[Dashboard]) -> Dict[str, str]:
        """Copy dashboard JSON files and return mapping of filename to relative path."""
        dashboards_dir = grafana_dir / "dashboards"
        dashboards_dir.mkdir(parents=True, exist_ok=True)

        dashboard_paths = {}

        for dashboard in dashboards:
            # Preserve relative path structure
            dest_path = dashboards_dir / dashboard.relative_path
            dest_path.parent.mkdir(parents=True, exist_ok=True)

            copy_file(dashboard.file_path, dest_path)

            # Store relative path from grafana/ directory
            rel_path = f"dashboards/{dashboard.relative_path}"
            dashboard_paths[dashboard.filename] = rel_path

            if self.verbose:
                print(f"  Copied dashboard: {dashboard.relative_path}")

        return dashboard_paths

    def _collect_folders(self, dashboards: List[Dashboard]) -> Set[str]:
        """Collect unique folder UIDs from dashboards."""
        folders = set()
        for dashboard in dashboards:
            if dashboard.folder_uid:
                folders.add(dashboard.folder_uid)
        return folders

    def _generate_resources_file(
        self,
        grafana_dir: Path,
        dashboards: List[Dashboard],
        folders: Set[str],
        dashboard_paths: Dict[str, str],
        home_dashboard: str = None
    ):
        """Generate main resources.gfo.yaml file."""
        resources = {}

        # Add folders section
        if folders:
            folder_list = []
            for folder_uid in sorted(folders):
                # Extract readable name from UID (hdx-<name>-folder -> <name>)
                folder_name = folder_uid.replace('hdx-', '').replace('-folder', '').replace('-', ' ').title()
                folder_list.append({
                    'uid': folder_uid,
                    'title': folder_name
                })
            resources['folders'] = folder_list

        # Add dashboards section
        dashboard_list = []
        for dashboard in dashboards:
            dashboard_entry = {
                'file': dashboard_paths[dashboard.filename],
                'folder_uid': dashboard.folder_uid
            }

            # Add inputs if present
            if dashboard.inputs:
                inputs = []
                for inp in dashboard.inputs:
                    input_entry = {
                        'name': inp.name,
                        'type': inp.type
                    }
                    if inp.value is not None:
                        input_entry['value'] = inp.value
                    inputs.append(input_entry)
                dashboard_entry['inputs'] = inputs

            # Mark as home dashboard if specified
            if home_dashboard and dashboard.filename == home_dashboard:
                dashboard_entry['home'] = True

            dashboard_list.append(dashboard_entry)

        resources['dashboards'] = dashboard_list

        # Write resources file
        resources_path = grafana_dir / "resources.gfo.yaml"
        write_file(resources_path, dump_yaml(resources, sort_keys=False))

        if self.verbose:
            print(f"  Generated resources.gfo.yaml")
