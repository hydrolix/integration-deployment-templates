# Hydrolix Bundle Deployer

## What is a Bundle?

A Hydrolix Bundle is a structured JSON configuration file that defines a complete data integration package for the Hydrolix platform. It serves as a blueprint for connecting data sources, transforming data, creating visualizations, managing dependencies, and configuring alert rules.

The specification of the Bundle format is located here: [BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md)

Instructions on how to use validation and deployment tools is here: [HOW-TO-TEST.md](./HOW-TO-TEST.md)

An explanation of what is verified during the validation process is here: [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md)

## Key Components

- **Data Source Configuration** - Defines where data originates (MCDN, CloudFront, WAF, etc.)
- **Integration Method** - Specifies how data is ingested (HTTP Streaming, Kinesis, S3, etc.)
- **Transformations** - Data processing and enrichment rules with sample data
- **Dashboard Templates** - Pre-built visualizations (Grafana dashboards)
- **Alert Rules** - Proactive monitoring and alerting configurations
- **Functions & Dictionaries** - Custom SQL functions and lookup tables for data enrichment
- **Summary Tables** - Pre-aggregated views for faster queries
- **Metadata** - Versioning, maintainer information, and descriptions
- **Dependencies** - Required functions, dictionaries, and external resources

## Why Bundles are Important

### 1. **Standardization**
- Provides consistent structure across all integrations
- Ensures compatibility with Hydrolix platform components
- Enables automated validation and deployment
- Template variables (`__PROJECT_NAME__`) make bundles portable across environments

### 2. **Reusability**
- Packages complex integrations as shareable components
- Allows teams to reuse battle-tested configurations
- Auto-discovery of functions and dictionaries reduces configuration burden
- Reduces implementation time for common use cases

### 3. **Dependency Management**
- Explicitly defines required functions and dictionaries
- Manages external resources (GeoIP databases, user-agent parsers)
- Handles large dictionary files through zip extraction
- Prevents version conflicts and compatibility issues

### 4. **Validation & Safety**
- Built-in validation prevents misconfigurations
- Security checks ensure proper data validation
- Production mode validates dependencies exist before deployment
- Checksum verification ensures file integrity

### 5. **Documentation**
- Self-documenting structure
- Clear metadata about purpose and maintainers
- Version tracking for updates and maintenance
- Inline sample data for testing and validation

### 6. **Automation**
- Automated deployment orchestration
- Headless browser testing for dashboard validation
- Bundle-aware cleanup for safe resource management
- Auto-discovery mode for zero-configuration deployments

## Quick Start

### Prerequisites
- **Deno** runtime installed ([deno.land](https://deno.land/))
- **Docker** for local Grafana testing
- **Chrome/Chromium** for headless browser testing
- **Environment Variables**:
  ```bash
  export BUNDLE_TESTING_CLUSTER="your-cluster.domain.com"
  export BUNDLE_TESTING_USERNAME="your-username"
  export BUNDLE_TESTING_PASSWORD="your-password"
  ```

### Basic Commands

```bash
# Validate a bundle (fast, no deployment)
deno run --allow-all src/main.ts mcdn_test

# Deploy with local Grafana testing
deno run --allow-all src/main.ts --local mcdn_test

# Deploy dashboard only (no tables/data)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Clean up bundle resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# Dry run cleanup (see what would be deleted)
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
```

## Template Variables System

Bundles use template variables to ensure portability across projects and environments:

### Core Variables
- `__PROJECT_NAME__` - Replaced with actual project name (e.g., `sample_project`)
- `__DATASOURCE__` - Replaced with Grafana datasource UID
- `__TABLE_NAME__` - Replaced with table names
- `__DASHBOARD_UUID__` - Replaced with unique dashboard identifiers

### Why Template Variables Matter

**In function definitions** (`functions/city_name.json`):
```json
{
  "sql": "(ip) -> dictGetString('__PROJECT_NAME___geoip_dict', 'city', ip)"
}
```

**Deployed as:**
```sql
-- Becomes: sample_project_city_name (Hydrolix adds prefix)
-- References: sample_project_geoip_dict (also prefixed)
SELECT sample_project_city_name(client_ip) AS city
```

This ensures all resource references match the actual deployed names, preventing runtime errors.

## Auto-Discovery Features

The Bundle Deployer can automatically discover and deploy resources:

### Explicit Mode (Recommended)
```json
{
  "dependencies": {
    "hydrolix": {
      "required_functions": ["city_name", "breadcrumbs"],
      "required_dictionaries": ["ua_cat_dict", "geoip_city_blocks"]
    }
  }
}
```

### Auto-Discovery Mode (Zero Config)
```json
{
  "dependencies": {
    "hydrolix": {}  // Leave empty!
  }
}
```

The tool automatically:
- Scans `functions/` for all `.json` files
- Extracts `dictionaries.zip` and scans for all `.json` + data file pairs
- Deploys everything it finds

## Architecture Highlights

### Modular Validation System
- Each validation rule is a separate module in `src/validation/`
- Easy to add new validation rules
- Clear separation of concerns

### Deployment Orchestration
- **deploy.ts** - Full deployment (tables, transforms, data, dashboards, alerts)
- **deploy_only_dashboard.ts** - Dashboard-only deployment
- **hdx.ts** - Hydrolix API client with retry logic
- **grafana/interface.ts** - Grafana API client for dashboards and alert rules

### Production Safety
- Production mode (`--production`) validates dependencies without deploying
- Bundle-aware cleanup prevents accidental deletions
- Dry-run mode for safe testing
- Checksum verification for file integrity

## Bundle Structure

```
mcdn_test/
├── bundle.json                         # Manifest
├── functions/                          # Custom SQL functions
│   ├── city_name.json
│   └── breadcrumbs.json
├── dictionaries/                       # Lookup tables
│   ├── dictionaries.zip               # Large files (auto-extracted)
│   ├── .extracted/                    # Auto-created (gitignored)
│   ├── custom_dict.json               # Optional overrides
│   └── custom_dict.csv
├── transformations/                    # Data parsing schemas
│   ├── mcdn_akamai_ds2.json
│   └── mcdn_cloudflare.json
├── dashboards/                         # Grafana visualizations
│   ├── CDN Dashboard.json
│   ├── alert-rules.json               # Alert configurations
│   └── Raw Logs.json                  # Additional dashboards
└── summaries/                          # Pre-aggregated views
    ├── mcdn_summary_min.sql
    └── mcdn_summary_hour.sql
```

## Key Features

### 1. **Zip Extraction for Large Files**
Automatically extracts `dictionaries.zip` to handle files exceeding GitHub's limits:
```
dictionaries/
├── dictionaries.zip              # Committed to git
└── .extracted/                   # Auto-created, gitignored
    ├── ua_cat_dict.json
    └── ua_cat_dict.yaml
```

### 2. **Alert Rules Support**
Deploy alert rules alongside dashboards:
```json
{
  "alert_rules": {
    "path": "dashboards/alert-rules.json"
  }
}
```

### 3. **Summary Tables**
Create pre-aggregated views for faster queries:
```json
{
  "summary_tables": [
    {
      "name": "mcdn_summary_min",
      "parent_table_name": "mcdn_test",
      "dashboard_var": "__SUMMARY_TABLE_NAME_1__",
      "sql": {"path": "summaries/mcdn_summary_min.sql"}
    }
  ]
}
```

### 4. **Multiple Dashboards**
Deploy primary and additional dashboards:
```json
{
  "dashboard": {
    "path": "dashboards/CDN Dashboard.json",
    "project_var": "__PROJECT_NAME__"
  },
  "other_dashboards": [
    {
      "path": "dashboards/Raw Logs.json",
      "project_var": "__PROJECT_NAME__"
    }
  ]
}
```

### 5. **Bundle-Aware Cleanup**
Safely delete only your bundle's resources:
```bash
# Delete only mcdn_test resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# Preview what would be deleted
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
```

## Testing & Validation

The Bundle Deployer performs comprehensive validation before deployment:

- ✅ Bundle structure and JSON validity
- ✅ File existence and accessibility
- ✅ Template variable presence
- ✅ SQL reference validation
- ✅ Function and dictionary file checks
- ✅ Transform schema validation
- ✅ Sample data verification
- ✅ Dashboard structure validation
- ✅ Alert rules format validation
- ✅ Headless browser testing (with `--local`)

See [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) for complete details.

## Documentation

- **[BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md)** - Complete bundle format specification
- **[HOW-TO-TEST.md](./HOW-TO-TEST.md)** - Testing and deployment guide
- **[WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md)** - Validation rules reference
- **[Bundle Deployer.md](./Bundle%20Deployer.md)** - Comprehensive user guide

## Contributing

When creating new bundles:
1. Copy an existing bundle as a template
2. Update `bundle.json` with your configuration
3. Add your functions, dictionaries, and transforms
4. Use `__PROJECT_NAME__` for all custom resource references
5. Validate with `deno run --allow-all src/main.ts your_bundle`
6. Test locally with `deno run --allow-all src/main.ts --local your_bundle`

## Support

For issues or questions:
- Review the documentation in this directory
- Check validation output for specific error messages
- Use cleanup tool for fresh starts: `deno run --allow-all src/cleanup.ts --all your_bundle`
- Consult [Hydrolix Documentation](https://docs.hydrolix.io/) for platform details