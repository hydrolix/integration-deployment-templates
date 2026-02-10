#!/usr/bin/env python3
"""Convert raw bundle assets to YAML-based CaC bundle format.

This script takes raw bundle assets (transforms, dashboards, summaries) and converts
them to the YAML-based Configuration as Code format for use in portable bundles.

Usage:
    python scripts/bundle_to_yaml.py \
        --source aws/cloudflare \
        --customer-type aws \
        --bundle-name cloudflare \
        --version 1.0.0 \
        --description "Cloudflare logs integration" \
        --maintainer "Hydrolix Team <team@hydrolix.io>"

The script will:
1. Discover assets in the source directory
2. Validate all inputs
3. Generate YAML bundle structure in portables/ directory
4. Validate outputs
"""

import argparse
import sys
from pathlib import Path

# Add scripts directory to path for imports when running script directly
script_dir = Path(__file__).parent
if str(script_dir) not in sys.path:
    sys.path.insert(0, str(script_dir))

from converters.discoverer import AssetDiscoverer
from converters.manifest_gen import ManifestGenerator
from converters.hydrolix_gen import HydrolixGenerator
from converters.grafana_gen import GrafanaGenerator
from converters.validator import BundleValidator
from utils.models import BundleMetadata


def auto_detect_from_path(source_path: str):
    """Auto-detect customer_type and bundle_name from source path.

    Args:
        source_path: Source path like "aws/cloudflare" or "trafficpeak/security"

    Returns:
        Tuple of (customer_type, bundle_name)
    """
    parts = Path(source_path).parts
    if len(parts) >= 2:
        return parts[0], parts[1]
    elif len(parts) == 1:
        return parts[0], parts[0]
    return None, None


def parse_args():
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Convert raw bundle assets to YAML-based CaC bundle format",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Minimal - auto-detects customer_type and bundle_name from path
  python scripts/bundle_to_yaml.py --source aws/cloudflare

  # With version and description
  python scripts/bundle_to_yaml.py \\
      --source aws/cloudflare \\
      --version 1.0.0 \\
      --description "Cloudflare logs integration"

  # Override auto-detected values
  python scripts/bundle_to_yaml.py \\
      --source aws/cloudflare \\
      --customer-type aws \\
      --bundle-name cloudflare-custom \\
      --version 1.0.0 \\
      --description "Cloudflare logs integration" \\
      --maintainer "Custom Team <custom@example.com>"
        """
    )

    parser.add_argument(
        '--source',
        required=True,
        help='Source directory containing raw bundle assets (e.g., aws/cloudflare)'
    )

    parser.add_argument(
        '--customer-type',
        help='Customer type (auto-detected from source path if not provided)'
    )

    parser.add_argument(
        '--bundle-name',
        help='Bundle name (auto-detected from source path if not provided)'
    )

    parser.add_argument(
        '--version',
        default='1.0',
        help='Bundle version (e.g., 1.0 or 1.0.0) (default: 1.0)'
    )

    parser.add_argument(
        '--description',
        help='Bundle description (auto-generated if not provided)'
    )

    parser.add_argument(
        '--maintainer',
        default='Hydrolix Team <team@hydrolix.io>',
        help='Bundle maintainer (default: Hydrolix Team <team@hydrolix.io>)'
    )

    parser.add_argument(
        '--table-name',
        help='Table name for Hydrolix resources (auto-detected based on customer type if not provided)'
    )

    parser.add_argument(
        '--home-dashboard',
        help='Filename of the home dashboard (optional)'
    )

    parser.add_argument(
        '--output',
        help='Output directory (default: portables/<customer_type>_<bundle_name>)'
    )

    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Enable verbose output'
    )

    parser.add_argument(
        '--skip-validation',
        action='store_true',
        help='Skip input/output validation (not recommended)'
    )

    return parser.parse_args()


def main():
    """Main conversion workflow."""
    args = parse_args()

    # Auto-detect customer_type and bundle_name from source path if not provided
    if not args.customer_type or not args.bundle_name:
        detected_customer, detected_bundle = auto_detect_from_path(args.source)
        if not args.customer_type:
            args.customer_type = detected_customer
        if not args.bundle_name:
            args.bundle_name = detected_bundle

    # Validate that we have required values
    if not args.customer_type or not args.bundle_name:
        print("❌ Error: Could not auto-detect customer_type and bundle_name from source path.")
        print("   Please provide --customer-type and --bundle-name explicitly.")
        sys.exit(1)

    # Auto-generate description if not provided
    if not args.description:
        args.description = f"{args.customer_type.upper()} {args.bundle_name.replace('-', ' ').replace('_', ' ').title()} integration bundle"

    # Auto-detect table_name based on customer_type if not provided
    if not args.table_name:
        if args.customer_type == 'trafficpeak':
            args.table_name = 'akamai_logs'
        else:
            args.table_name = 'logs'

    # Resolve paths
    repo_root = Path(__file__).parent.parent
    source_path = repo_root / args.source

    if args.output:
        output_path = Path(args.output)
    else:
        # Default output: <bundle_name>/<version>
        output_path = repo_root / args.bundle_name / args.version

    if args.verbose:
        print(f"Source: {source_path}")
        print(f"Output: {output_path}")
        print(f"Customer Type: {args.customer_type}")
        print(f"Bundle Name: {args.bundle_name}")
        print(f"Table Name: {args.table_name}")
        print()

    # Build metadata
    metadata = BundleMetadata(
        customer_type=args.customer_type,
        bundle_name=args.bundle_name,
        version=args.version,
        description=args.description,
        maintainer=args.maintainer,
        table_name=args.table_name,
        home_dashboard=args.home_dashboard
    )

    # Initialize components
    validator = BundleValidator(verbose=args.verbose)
    discoverer = AssetDiscoverer(source_path, verbose=args.verbose)
    manifest_gen = ManifestGenerator(verbose=args.verbose)
    hydrolix_gen = HydrolixGenerator(verbose=args.verbose)
    grafana_gen = GrafanaGenerator(verbose=args.verbose)

    print("=" * 60)
    print(f"Converting Bundle: {args.customer_type}/{args.bundle_name}")
    print("=" * 60)
    print()

    # Phase 1: Discover assets
    print("Phase 1: Discovering assets...")
    assets = discoverer.discover()

    if args.verbose:
        print(f"  Found {len(assets.transforms)} transform(s)")
        print(f"  Found {len(assets.dashboards)} dashboard(s)")
        print(f"  Found {len(assets.summaries)} summary/summaries")
        print()

    # Phase 2: Validate inputs
    if not args.skip_validation:
        print("Phase 2: Validating inputs...")

        # Validate assets
        valid_assets, asset_errors = validator.validate_input(source_path, assets)
        if not valid_assets:
            print("\n❌ Asset validation failed:")
            for error in asset_errors:
                print(f"  • {error}")
            sys.exit(1)

        # Validate metadata
        valid_metadata, metadata_errors = validator.validate_metadata(metadata)
        if not valid_metadata:
            print("\n❌ Metadata validation failed:")
            for error in metadata_errors:
                print(f"  • {error}")
            sys.exit(1)

        print()

    # Phase 3: Generate bundle
    print("Phase 3: Generating YAML bundle...")

    # Create output directory
    output_path.mkdir(parents=True, exist_ok=True)

    # Generate manifest
    manifest_gen.write(output_path, metadata)

    # Generate Hydrolix resources
    hydrolix_gen.generate(output_path, assets, args.table_name)

    # Generate Grafana resources
    grafana_gen.generate(output_path, assets, args.home_dashboard)

    print()

    # Phase 4: Validate outputs
    if not args.skip_validation:
        print("Phase 4: Validating outputs...")
        valid_output, output_errors = validator.validate_output(output_path)
        if not valid_output:
            print("\n❌ Output validation failed:")
            for error in output_errors:
                print(f"  • {error}")
            sys.exit(1)
        print()

    # Success
    print("=" * 60)
    print("✓ Conversion completed successfully!")
    print("=" * 60)
    print()
    print(f"Bundle generated at: {output_path}")
    print()
    print("Structure:")
    print(f"  {output_path}/")
    print(f"  ├── {args.bundle_name}.bdl.yaml")
    if assets.transforms or assets.summaries:
        print(f"  ├── hydrolix/")
        print(f"  │   └── resources.hdp.yaml")
    if assets.dashboards:
        print(f"  └── grafana/")
        print(f"      └── resources.gfo.yaml")
    print()


if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nConversion cancelled by user.")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        if '--verbose' in sys.argv:
            raise
        sys.exit(1)
