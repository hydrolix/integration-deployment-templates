# Bundle Conversion Scripts

This directory contains utilities for converting Hydrolix integration bundles between different formats.

## bundle_to_yaml.py

Converts raw bundle assets (transforms, dashboards, summaries) to YAML-based Configuration as Code format for portable bundles.

### Usage

```bash
python scripts/bundle_to_yaml.py \
    --source <source-path> \
    --customer-type <customer-type> \
    --bundle-name <bundle-name> \
    --version <version> \
    --description "<description>" \
    --maintainer "<maintainer-info>"
```

### Required Arguments

- `--source`: Source directory containing raw bundle assets (e.g., `aws/cloudflare`)
- `--customer-type`: Customer type (e.g., `aws`, `gcp`, `azure`, `trafficpeak`)
- `--bundle-name`: Bundle name (e.g., `cloudflare`, `waf`, `bot-detection`)
- `--version`: Bundle version in semantic versioning format (e.g., `1.0.0`)
- `--description`: Bundle description
- `--maintainer`: Bundle maintainer (e.g., `"Hydrolix Team <team@hydrolix.io>"`)

### Optional Arguments

- `--table-name`: Table name for Hydrolix resources (default: `logs`)
- `--home-dashboard`: Filename of the home dashboard
- `--output`: Output directory (default: `portables/<customer_type>_<bundle_name>`)
- `--verbose`: Enable verbose output
- `--skip-validation`: Skip input/output validation (not recommended)

### Examples

#### AWS CloudFlare Bundle

```bash
python scripts/bundle_to_yaml.py \
    --source aws/cloudflare \
    --customer-type aws \
    --bundle-name cloudflare \
    --version 1.0.0 \
    --description "Cloudflare logs integration" \
    --maintainer "Hydrolix Team <team@hydrolix.io>"
```

#### TrafficPeak Security Bundle

```bash
python scripts/bundle_to_yaml.py \
    --source trafficpeak/security \
    --customer-type trafficpeak \
    --bundle-name security \
    --version 1.0.0 \
    --table-name akamai_logs \
    --description "Akamai security logs" \
    --maintainer "Hydrolix Team <team@hydrolix.io>"
```

#### With Custom Output and Home Dashboard

```bash
python scripts/bundle_to_yaml.py \
    --source aws/bot-detection \
    --customer-type aws \
    --bundle-name bot-detection \
    --version 1.0.0 \
    --description "Bot detection integration" \
    --maintainer "Hydrolix Team <team@hydrolix.io>" \
    --home-dashboard "overview.json" \
    --output custom/output/path \
    --verbose
```

### Input Structure

The script expects raw bundle assets in the source directory:

```
aws/cloudflare/
├── transformations/          # or transforms/
│   └── transform.json
├── dashboards/              # or grafana/
│   ├── overview.json
│   └── details.json
└── summaries/               # optional
    └── hourly_stats.sql
```

### Output Structure

The script generates a YAML-based bundle in the output directory:

```
portables/aws_cloudflare/
├── cloudflare.bdl.yaml                  # Bundle manifest
├── hydrolix/
│   ├── resources.hdp.yaml               # Hydrolix resources index
│   ├── tables/
│   │   └── logs.hdx.yaml                # Table definition
│   ├── transforms/
│   │   └── cloudflare.json              # Transform (copied from source)
│   ├── summaries/                       # If summaries exist
│   │   └── hourly_stats.hdx.yaml
│   └── sql/
│       └── hourly_stats.sql
└── grafana/
    ├── resources.gfo.yaml               # Grafana resources index
    └── dashboards/
        ├── overview.json
        └── details.json
```

### Conversion Process

1. **Discovery**: Scans source directory for assets
   - Finds transforms in `transformations/` or `transforms/`
   - Finds dashboards in `dashboards/` or `grafana/`
   - Finds summaries in `summaries/`

2. **Validation**: Validates inputs
   - Checks all JSON files are valid
   - Verifies required metadata fields
   - Ensures semantic version format

3. **Generation**: Creates YAML bundle structure
   - Generates bundle manifest (`.bdl.yaml`)
   - Creates Hydrolix resources with `__extend__` references
   - Organizes Grafana dashboards with folder structure
   - Extracts dashboard inputs from `__inputs` array

4. **Validation**: Validates outputs
   - Checks all required files were generated
   - Verifies directory structure

## Module Structure

```
scripts/
├── bundle_to_yaml.py           # Main CLI entry point
├── converters/
│   ├── discoverer.py          # Asset discovery
│   ├── manifest_gen.py        # Manifest generation
│   ├── hydrolix_gen.py        # Hydrolix resources generation
│   ├── grafana_gen.py         # Grafana resources generation
│   └── validator.py           # Input/output validation
└── utils/
    ├── models.py              # Data models
    ├── file_utils.py          # File operations
    └── yaml_utils.py          # YAML formatting
```

## Integration with GitHub Workflows

This script can be integrated into GitHub workflows for automated bundle conversion:

```yaml
- name: Convert bundle to YAML
  run: |
    python scripts/bundle_to_yaml.py \
      --source aws/cloudflare \
      --customer-type aws \
      --bundle-name cloudflare \
      --version ${{ github.ref_name }} \
      --description "Cloudflare logs integration" \
      --maintainer "Hydrolix Team <team@hydrolix.io>"
```

## Requirements

- Python 3.7+
- PyYAML (for YAML generation)

Install dependencies:

```bash
pip install pyyaml
```
