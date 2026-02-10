"""Bundle manifest generator."""

from pathlib import Path

from utils.models import BundleMetadata
from utils.yaml_utils import dump_yaml
from utils.file_utils import write_file


class ManifestGenerator:
    """Generates bundle manifest file (.bdl.yaml)."""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose

    def generate(self, metadata: BundleMetadata) -> str:
        """Generate .bdl.yaml content."""
        manifest = {
            'name': f"{metadata.customer_type}_{metadata.bundle_name}",
            'version': metadata.version,
            'description': metadata.description,
            'maintainer': metadata.maintainer,
            'hydrolix': {
                'resources': 'hydrolix/resources.hdp.yaml'
            },
            'grafana': {
                'resources': 'grafana/resources.gfo.yaml'
            }
        }

        return dump_yaml(manifest, sort_keys=False)

    def write(self, output_path: Path, metadata: BundleMetadata):
        """Generate and write manifest file."""
        filename = f"{metadata.bundle_name}.bdl.yaml"
        content = self.generate(metadata)

        file_path = output_path / filename
        write_file(file_path, content)

        if self.verbose:
            print(f"✓ Generated {filename}")

        return file_path
