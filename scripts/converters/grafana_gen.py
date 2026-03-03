"""Grafana resource generator."""

from pathlib import Path
from typing import List, Dict

from utils.models import BundleAssets, Dashboard
from utils.yaml_utils import dump_yaml
from utils.file_utils import write_file, copy_file, sanitize_filename


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

        # Generate main resources file
        self._generate_resources_file(grafana_dir, assets.dashboards, dashboard_paths, home_dashboard)

        if self.verbose:
            print(f"✓ Generated Grafana resources")

    def _copy_dashboards(self, grafana_dir: Path, dashboards: List[Dashboard]) -> Dict[str, str]:
        """Copy dashboard JSON files and return mapping of filename to relative path."""
        dashboards_dir = grafana_dir / "dashboards"
        dashboards_dir.mkdir(parents=True, exist_ok=True)

        dashboard_paths = {}

        for dashboard in dashboards:
            # Use just the filename (flatten structure)
            dest_path = dashboards_dir / dashboard.filename

            copy_file(dashboard.file_path, dest_path)

            # Store relative path from grafana/ directory
            rel_path = f"dashboards/{dashboard.filename}"
            dashboard_paths[dashboard.filename] = rel_path

            if self.verbose:
                print(f"  Copied dashboard: {dashboard.filename}")

        return dashboard_paths

    def _generate_dashboard_uid(self, filename: str) -> str:
        """Generate dashboard UID from filename."""
        # Remove extension and sanitize
        name = Path(filename).stem
        sanitized = sanitize_filename(name.lower())
        return f"hdx-{sanitized}"

    def _generate_resources_file(
        self,
        grafana_dir: Path,
        dashboards: List[Dashboard],
        dashboard_paths: Dict[str, str],
        home_dashboard: str = None
    ):
        """Generate main resources.gfo.yaml file with nested structure."""
        resources = {}

        # Build nested folder structure
        folders_dict = {
            'hdx-main-folder': {
                'name': 'TrafficPeak Certified Reference Dashboards',
                'children': {}
            }
        }

        # Collect child folders
        child_folders = set()
        for dashboard in dashboards:
            if dashboard.folder_uid and dashboard.folder_uid != 'hdx-main-folder':
                child_folders.add(dashboard.folder_uid)

        # Add child folders to main folder
        for folder_uid in sorted(child_folders):
            # Extract readable name from UID (hdx-<name>-folder -> <name>)
            folder_name = folder_uid.replace('hdx-', '').replace('-folder', '').replace('-', ' ').title()
            folders_dict['hdx-main-folder']['children'][folder_uid] = {
                'name': folder_name
            }

        resources['folders'] = folders_dict

        # Build dashboards dict
        dashboards_dict = {}
        for dashboard in dashboards:
            dashboard_uid = self._generate_dashboard_uid(dashboard.filename)

            dashboard_entry = {
                'dashboard': {
                    '__extend__': dashboard_paths[dashboard.filename]
                },
                'folderUid': dashboard.folder_uid
            }

            # Add inputs if present - as a flat dict
            if dashboard.inputs:
                inputs_dict = {}
                for inp in dashboard.inputs:
                    # Use the input name as the key, and value/type as the value
                    # For datasource inputs, use the predefined datasource UID
                    if inp.type == 'datasource':
                        inputs_dict[inp.name] = 'hdx-hydrolix-datasource'
                    elif inp.value is not None:
                        inputs_dict[inp.name] = inp.value
                    else:
                        inputs_dict[inp.name] = ''
                dashboard_entry['inputs'] = inputs_dict

            # Mark as home dashboard if specified
            if home_dashboard and dashboard.filename == home_dashboard:
                dashboard_entry['home'] = True

            dashboards_dict[dashboard_uid] = dashboard_entry

        resources['dashboards'] = dashboards_dict

        # Write resources file
        resources_path = grafana_dir / "resources.gfo.yaml"
        write_file(resources_path, dump_yaml(resources, sort_keys=False))

        if self.verbose:
            print(f"  Generated resources.gfo.yaml")
