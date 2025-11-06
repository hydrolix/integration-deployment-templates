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
- **Shared Resources** - Common functions and dictionaries shared across all bundles in `hdx_solutions` project
- **Metadata** - Versioning, maintainer information, and descriptions
- **Dependencies** - Required functions, dictionaries, and external resources

## Shared Resources

The Bundle Deployer supports shared resources that live in a central `hdx_solutions` project:

- **Shared Functions** - Common SQL functions used across multiple bundles (GeoIP lookups, user agent parsing, etc.)
- **Shared Dictionaries** - Large reference data shared by all bundles (GeoIP databases, user agent categories, etc.)
- **Bundle-Specific Resources** - Functions and dictionaries unique to a specific bundle

**Benefits:**
- ✅ No duplication - shared resources created once, used by all bundles
- ✅ Easier maintenance - update shared resources in one centralized location
- ✅ Smaller bundles - don't package common dictionaries repeatedly
- ✅ Consistent behavior - all bundles use same GeoIP/UA parsing logic
- ✅ Faster deployments - skip creation of resources that already exist

**Resource Projects:**
- **hdx_solutions** - Shared resources (e.g., `hdx_solutions_city_name`, `hdx_solutions_geoip_city`)
- **sample_project** - Bundle-specific resources (e.g., `sample_project_custom_parser`)

## Why Bundles are Important

### 1. **Standardization**
- Provides consistent structure across all integrations
- Ensures compatibility with Hydrolix platform components
- Enables automated validation and deployment
- Template variables (`__PROJECT_NAME__`, `__SHARED_PROJECT__`) make bundles portable across environments

### 2. **Reusability**
- Packages complex integrations as shareable components
- Allows teams to reuse battle-tested configurations
- Auto-discovery of functions and dictionaries reduces configuration burden
- Reduces implementation time for common use cases

### 3. **Dependency Management**
- Explicitly defines required functions and dictionaries
- Separates shared vs. bundle-specific dependencies
- Manages external resources (GeoIP databases, user-agent parsers)
- Handles large dictionary files through zip extraction
- Prevents version conflicts and compatibility issues

### 4. **Validation & Safety**
- Built-in validation prevents misconfigurations
- Security checks ensure proper data validation
- Production mode validates dependencies exist before deployment
- Checksum verification ensures file integrity
- Fail-fast behavior catches missing dependencies before runtime

### 5. **Documentation**
- Self-documenting structure
- Clear metadata about purpose and maintainers
- Version tracking for updates and maintenance
- Inline sample data for testing and validation
- Explicit declaration of all resource dependencies

### 6. **Automation**
- Automated deployment orchestration
- Headless browser testing for dashboard validation
- Bundle-aware cleanup for safe resource management
- Auto-discovery mode for zero-configuration deployments

### 7. **Resource Sharing**
- Shared functions and dictionaries reduce duplication across bundles
- Common resources (GeoIP, user agents) maintained centrally in `hdx_solutions`
- Bundle-specific resources for unique transformation logic
- Cross-project references enable flexible architectures
- Automatic resource discovery for vendor-provided bundles

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
  
  # Optional: Override shared project name (defaults to hdx_solutions)
  export SHARED_PROJECT_NAME="hdx_solutions"
  ```

### Basic Commands

```bash
# Validate a bundle (fast, no deployment)
deno run --allow-all src/main.ts mcdn_test

# Deploy with local Grafana testing
deno run --allow-all src/main.ts --local mcdn_test

# Deploy with strict plugin validation (fails if plugins missing)
deno run --allow-all src/main.ts --local --strict-plugins mcdn_test

# Deploy dashboard only (no tables/data)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Clean up bundle resources (shared resources preserved)
deno run --allow-all src/cleanup.ts --all mcdn_test

# Dry run cleanup (see what would be deleted)
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
```

## Template Variables System

Bundles use template variables to ensure portability across projects and environments:

### Core Variables
- `__PROJECT_NAME__` - Replaced with bundle project name (e.g., `sample_project`)
- `__SHARED_PROJECT__` - Replaced with shared project name (default: `hdx_solutions`)
- `__DATASOURCE__` - Replaced with Grafana datasource UID
- `__TABLE_NAME__` - Replaced with table names
- `__DASHBOARD_UUID__` - Replaced with unique dashboard identifiers

### Why Template Variables Matter

**In shared function definitions** (`functions/city_name.json`):
```json
{
  "sql": "(ip) -> dictGetString('__SHARED_PROJECT___geoip_dict', 'city', ip)"
}
```

**In bundle-specific function definitions** (`functions/custom_parser.json`):
```json
{
  "sql": "(data) -> dictGet('__PROJECT_NAME___custom_dict', 'value', data)"
}
```

**Deployed as:**
```sql
-- Shared function: hdx_solutions_city_name (in hdx_solutions project)
-- Bundle function: sample_project_custom_parser (in sample_project)
SELECT hdx_solutions_city_name(client_ip) AS city,
       sample_project_custom_parser(data) AS parsed
```

This ensures all resource references match the actual deployed names, preventing runtime errors.

## Auto-Discovery Features

The Bundle Deployer can automatically discover and deploy resources:

### Explicit Mode (Recommended)
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name", "breadcrumbs"],
      "shared_dictionaries": ["geoip_city"],
      "required_dictionaries": ["ua_cat_dict"]
    }
  }
}
```

**What happens:**
- ✅ Creates shared resources in `hdx_solutions`
- ✅ Creates bundle-specific resources in `sample_project`
- ✅ Validates all declared resources have local files
- ✅ No auto-discovery

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
- Deploys everything as **bundle-specific** in `sample_project`
- No shared resources created

### Hybrid Mode
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name"],
      "shared_dictionaries": ["geoip_city"]
      // Omit required_* to auto-discover bundle-specific
    }
  }
}
```

**What happens:**
- ✅ Creates declared shared resources in `hdx_solutions`
- 🔍 Auto-discovers bundle-specific resources from filesystem
- ✅ Best of both worlds

### Important: Empty Arrays Disable Auto-Discovery

**Don't do this:**
```json
{
  "required_functions": [],  // ❌ Empty array disables auto-discovery
  "shared_functions": ["city_name"]
}
```

**Do this instead:**
```json
{
  // ✅ Omit required_functions entirely
  "shared_functions": ["city_name"]
}
```

## Architecture Highlights

### Modular Validation System
- Each validation rule is a separate module in `src/validation/`
- Easy to add new validation rules
- Clear separation of concerns

### Deployment Orchestration
- **deploy.ts** - Full deployment (tables, transforms, data, dashboards, alerts)
- **deploy_only_dashboard.ts** - Dashboard-only deployment
- **hdx.ts** - Hydrolix API client with retry logic for bundle-specific resources
- **hdx_shared.ts** - Shared resources management (hdx_solutions project)
- **grafana/interface.ts** - Grafana API client for dashboards and alert rules

### Production Safety
- Production mode (`--production`) validates dependencies without deploying
- Bundle-aware cleanup prevents accidental deletions (preserves shared resources)
- Dry-run mode for safe testing
- Checksum verification for file integrity
- Fail-fast validation for missing declared resources

## Bundle Structure

```
mcdn_test/
├── bundle.json                         # Manifest
├── functions/                          # Custom SQL functions
│   ├── city_name.json                 # Can be shared or bundle-specific
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

### 1. **Shared Resources**
Resources shared across all bundles in `hdx_solutions` project:
- Common GeoIP functions (city_name, country_iso_code, etc.)
- Large GeoIP dictionaries (city blocks, ASN blocks, etc.)
- User agent parsers and categorization
- Reused by all bundles, maintained centrally
- Created once, referenced by all bundles

### 2. **Zip Extraction for Large Files**
Automatically extracts `dictionaries.zip` to handle files exceeding GitHub's limits:
```
dictionaries/
├── dictionaries.zip              # Committed to git
└── .extracted/                   # Auto-created, gitignored
    ├── ua_cat_dict.json
    └── ua_cat_dict.yaml
```

### 3. **Alert Rules Support**
Deploy alert rules alongside dashboards:
```json
{
  "alert_rules": {
    "path": "dashboards/alert-rules.json"
  }
}
```

### 4. **Summary Tables**
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

### 5. **Multiple Dashboards**
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

### 6. **Bundle-Aware Cleanup**
Safely delete only your bundle's resources:
```bash
# Delete only mcdn_test bundle-specific resources (shared resources preserved)
deno run --allow-all src/cleanup.ts --all mcdn_test

# Preview what would be deleted
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
```

**Note:** Cleanup preserves shared resources in `hdx_solutions` that may be used by other bundles.

## Testing & Validation

The Bundle Deployer performs comprehensive validation before and after deployment:

- ✅ Bundle structure and JSON validity
- ✅ File existence and accessibility
- ✅ Template variable presence (including `__SHARED_PROJECT__`)
- ✅ SQL reference validation
- ✅ Function and dictionary file checks (shared and bundle-specific)
- ✅ Transform schema validation
- ✅ Sample data verification
- ✅ Dashboard structure validation
- ✅ Alert rules format validation
- ✅ Headless browser testing (with `--local`)
- ✅ Grafana plugin detection (runtime validation after deployment)
- ✅ Shared resources validation (declared resources have local files)

See [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) for complete details.

## Documentation

- **[BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md)** - Complete bundle format specification with shared resources
- **[HOW-TO-TEST.md](./HOW-TO-TEST.md)** - Testing and deployment guide
- **[WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md)** - Validation rules reference
- **[Bundle Deployer.md](./Bundle%20Deployer.md)** - Comprehensive user guide

## Contributing

When creating new bundles:
1. Copy an existing bundle as a template
2. Update `bundle.json` with your configuration
3. Declare shared vs. bundle-specific resources appropriately
4. Add your functions, dictionaries, and transforms
5. Use `__SHARED_PROJECT__` for shared resource references
6. Use `__PROJECT_NAME__` for bundle-specific resource references
7. Validate with `deno run --allow-all src/main.ts your_bundle`
8. Test locally with `deno run --allow-all src/main.ts --local your_bundle`

## Support

For issues or questions:
- Review the documentation in this directory
- Check validation output for specific error messages
- Use cleanup tool for fresh starts: `deno run --allow-all src/cleanup.ts --all your_bundle`
- Verify resources in correct project (hdx_solutions vs sample_project)
- Ensure template variables use correct prefix (`__SHARED_PROJECT__` vs `__PROJECT_NAME__`)
- Consult [Hydrolix Documentation](https://docs.hydrolix.io/) for platform details