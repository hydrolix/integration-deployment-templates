"""Grafana resource generator."""

import re
from pathlib import Path
from typing import List, Dict

from utils.models import BundleAssets, Dashboard
from utils.yaml_utils import dump_yaml
import json

from utils.file_utils import write_file, sanitize_filename, sanitize_cac_name, slugify_grafana_title

# Matches <uuid>/<slug> dashboard reference values in __inputs constants
_UUID_SLUG_RE = re.compile(
    r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}'
    r'/([a-z0-9][a-z0-9-]*)$'
)


class GrafanaGenerator:
    """Generates Grafana resources (dashboards, folders, resources.gfo.yaml)."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def generate(self, output_path: Path, assets: BundleAssets, home_dashboard: str = None, folder_path: list = None):
        """Generate all Grafana resources.

        Args:
            output_path: Path to bundle output directory
            assets: Discovered bundle assets
            home_dashboard: Optional home dashboard filename
            folder_path: Ordered list of path segments (category + bundle name) used to
                           build the nested Grafana folder hierarchy, e.g. ['security', 'ds2']
        """
        grafana_dir = output_path / "grafana"

        if not assets.dashboards:
            if self.verbose:
                print("  No dashboards found, skipping Grafana generation")
            return

        # Build slug→stable-uid map so sibling UID references in __inputs can be
        # resolved to the deterministic hdx-* keys the GFO tool assigns.
        sibling_uid_map = self._build_sibling_uid_map(assets.dashboards)

        # Copy dashboards
        dashboard_paths = self._copy_dashboards(grafana_dir, assets.dashboards, sibling_uid_map)

        # Generate main resources file
        self._generate_resources_file(grafana_dir, assets.dashboards, dashboard_paths, home_dashboard, folder_path or [], sibling_uid_map)

        if self.verbose:
            print(f"✓ Generated Grafana resources")

    def _build_sibling_uid_map(self, dashboards: List[Dashboard]) -> Dict[str, str]:
        """Return {title-slug: stable-hdx-uid} for every dashboard in the bundle.

        Used to replace hardcoded <vendor-uuid>/<slug> references in __inputs
        with the deterministic hdx-* UIDs that the GFO deployment tool assigns.
        """
        slug_to_uid = {}
        for dashboard in dashboards:
            stable_uid = self._generate_dashboard_uid(dashboard.filename)
            try:
                with open(dashboard.file_path, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                db = data.get('dashboard', data)
                title = db.get('title', '')
                if title:
                    slug = slugify_grafana_title(title)
                    if slug:
                        slug_to_uid[slug] = stable_uid
            except Exception:
                pass
        return slug_to_uid

    def _resolve_sibling_input(self, value: str, sibling_uid_map: Dict[str, str]) -> str:
        """If value is '<vendor-uuid>/<slug>' and slug matches a sibling, return '<stable-uid>/<slug>'."""
        if not value:
            return value
        m = _UUID_SLUG_RE.match(value)
        if not m:
            return value
        slug = m.group(1)
        stable_uid = sibling_uid_map.get(slug)
        return f"{stable_uid}/{slug}" if stable_uid else value

    def _replace_datasource_uids(self, obj):
        """Recursively replace datasource UIDs with Grafana variable reference."""
        grafana_internal = {'-- Grafana --', 'grafana', '-- Dashboard --', '-- Mixed --'}

        if isinstance(obj, dict):
            # Detect datasource context: dict with both 'type' and 'uid' keys
            if 'type' in obj and 'uid' in obj:
                uid_val = obj['uid']
                if isinstance(uid_val, str) and uid_val not in grafana_internal:
                    obj['uid'] = '${DS_HYDROLIX-HYDROLIX-DATASOURCE}'
            for value in obj.values():
                self._replace_datasource_uids(value)
        elif isinstance(obj, list):
            for item in obj:
                self._replace_datasource_uids(item)

    def _copy_dashboards(self, grafana_dir: Path, dashboards: List[Dashboard], sibling_uid_map: Dict[str, str]) -> Dict[str, str]:
        """Copy dashboard JSON files, replacing datasource UIDs."""
        dashboards_dir = grafana_dir / "dashboards"
        dashboards_dir.mkdir(parents=True, exist_ok=True)

        dashboard_paths = {}

        for dashboard in dashboards:
            # Read, replace datasource UIDs, write cleaned JSON
            with open(dashboard.file_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
            self._replace_datasource_uids(data)

            # Normalize __inputs: fix datasource names and resolve sibling UID references
            for inp in data.get('__inputs', []):
                if inp.get('type') == 'datasource':
                    inp['name'] = 'DS_HYDROLIX-HYDROLIX-DATASOURCE'
                    inp['label'] = 'Hydrolix'
                elif inp.get('type') == 'constant' and inp.get('value'):
                    inp['value'] = self._resolve_sibling_input(inp['value'], sibling_uid_map)

            # Strip top-level dashboard uid (CaC deployments assign their own)
            data.pop('uid', None)

            sanitized_name = sanitize_cac_name(Path(dashboard.filename).stem) + '.json'
            dest_path = dashboards_dir / sanitized_name
            with open(dest_path, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
                f.write('\n')

            # Store relative path from grafana/ directory (map original filename to sanitized path)
            rel_path = f"dashboards/{sanitized_name}"
            dashboard_paths[dashboard.filename] = rel_path

            if self.verbose:
                print(f"  Copied dashboard: {sanitized_name}")

        return dashboard_paths

    def _generate_dashboard_uid(self, filename: str) -> str:
        """Generate dashboard UID from filename."""
        # Remove extension and sanitize
        name = Path(filename).stem
        sanitized = sanitize_filename(name.lower())
        return f"hdx-{sanitized}"

    # Human-readable display names for each folder segment
    _FOLDER_NAMES = {
        'api-context': 'API Context',
        'cdn':         'CDN',
        'multi-cdn':   'Multi-CDN',
        'dns':         'DNS',
        'media':       'Media',
        'security':    'Security',
        'bots':        'Bots',
        'ds2':         'DS2',
        'siem':        'SIEM',
    }

    def _build_folder_hierarchy(self, folder_path: list):
        """Build nested Grafana folder hierarchy from category path segments.

        Always produces hdx-main-folder at the root. Each segment in folder_path
        becomes a child folder nested one level deeper than the previous.

        Args:
            folder_path: Ordered segments, e.g. ['security'] or ['security', 'bots']

        Returns:
            Tuple of (folders_dict, deepest_folder_uid)
        """
        main_folder = {'name': 'TrafficPeak Certified Reference Dashboards'}
        folders_dict = {'hdx-main-folder': main_folder}
        deepest_uid = 'hdx-main-folder'

        if folder_path:
            current = main_folder
            for segment in folder_path:
                uid = f"hdx-{segment}-folder"
                name = self._FOLDER_NAMES.get(segment, segment.replace('-', ' ').title())
                child = {'name': name}
                current.setdefault('children', {})[uid] = child
                current = child
                deepest_uid = uid

        return folders_dict, deepest_uid

    def _generate_resources_file(
        self,
        grafana_dir: Path,
        dashboards: List[Dashboard],
        dashboard_paths: Dict[str, str],
        home_dashboard: str = None,
        folder_path: list = None,
        sibling_uid_map: Dict[str, str] = None,
    ):
        """Generate main resources.gfo.yaml file with nested structure."""
        resources = {}

        folders_dict, deepest_folder_uid = self._build_folder_hierarchy(folder_path or [])
        resources['folders'] = folders_dict

        # Build dashboards dict
        dashboards_dict = {}
        for dashboard in dashboards:
            dashboard_uid = self._generate_dashboard_uid(dashboard.filename)

            dashboard_entry = {
                'dashboard': {
                    '__extend__': dashboard_paths[dashboard.filename]
                },
                'folderUid': deepest_folder_uid
            }

            # Add inputs if present - as a flat dict
            if dashboard.inputs:
                inputs_dict = {}
                for inp in dashboard.inputs:
                    if inp.type == 'datasource':
                        inputs_dict['DS_HYDROLIX-HYDROLIX-DATASOURCE'] = 'hdx-hydrolix-datasource'
                    elif inp.value is not None:
                        value = inp.value
                        if sibling_uid_map:
                            value = self._resolve_sibling_input(value, sibling_uid_map)
                        inputs_dict[inp.name] = value
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