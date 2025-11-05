# Bundle Format Specification

A bundle is a Hydrolix JSON configuration file that packages transformations, dashboards, functions, dictionaries, and alert rules for data integration and visualization.

This document describes all valid fields and their validation rules for the TypeScript/Deno Bundle Deployer.

## Root Bundle Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | âœ… | Bundle identifier. Must contain only alphanumeric characters, underscores, and dashes |
| `source` | `string` | âœ… | Data source type. Must contain only alphanumeric characters, dashes, and underscores |
| `method` | `string` | âœ… | Integration method. See [Valid Methods](#valid-methods) |
| `beta` | `boolean` | âœ… | Whether this is a beta release |
| `base_url` | `string` | âœ… | HTTPS URL to the repository base path |
| `dashboard` | `Dashboard` | âœ… | Primary dashboard configuration |
| `other_dashboards` | `Dashboard[]` | âŒ | Optional additional dashboard configurations |
| `alert_rules` | `AlertRules` | âŒ | Optional alert rules configuration |
| `tables` | `Table[]` | âœ… | Array of table definitions |
| `summary_tables` | `SummaryTable[]` | âŒ | Optional array of summary table definitions |
| `ui` | `Ui` | âœ… | User interface configuration |
| `metadata` | `Metadata` | âœ… | Bundle metadata |
| `method_overrides` | `MethodOverrides` | âŒ | Optional method-specific overrides |
| `dependencies` | `Dependencies` | âŒ | Optional dependency requirements |

### Validation Rules for Root Object
- `base_url` must start with `https://` or `file://`
- `name` must contain only alphanumeric characters, underscores, and dashes
- `source` must contain only alphanumeric characters, dashes, and underscores
- No duplicate table names across all tables
- No duplicate `dashboard_var` values across all tables and summary tables
- `name` must include both `source` and `method` (case-insensitive)

### Base URL Format
Expected format: `https://github.com/hydrolix/integration-deployment-templates/blob/main/my-bundles/{bundle_name}`

---

## Dashboard Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | âœ… | Relative path to dashboard JSON file |
| `project_var` | `string` | âœ… | Variable placeholder for project name (must be `__PROJECT_NAME__`) |
| `sha256` | `string` | âŒ | Optional SHA256 hash of dashboard contents (64 hex characters) |

### Validation Rules for Dashboard
- `path` cannot start with `/` or contain `..`
- `path` must end with `.json`
- `project_var` must be `__PROJECT_NAME__`
- Dashboard JSON must contain required template variables:
  - `__DASHBOARD_UUID__`, `__DATASOURCE__`, `__PROJECT_NAME__`
  - All table `dashboard_var` values
  - All summary table `dashboard_var` values (if defined)
- Dashboard must have top-level `dashboard` object
- Dashboard must not have hardcoded `id` field
- Use `openssl dgst -sha256 <file_name>` to generate sha256

---

## AlertRules Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | âœ… | Relative path to alert rules JSON file |
| `sha256` | `string` | âŒ | Optional SHA256 hash (64 hex characters) |

### Validation Rules for AlertRules
- Path cannot start with `/` or contain `..`, must end with `.json`
- Must contain `apiVersion`, `groups` array with â‰¥1 group
- Each group must have: `name`, `folder`, `interval`, `rules` (â‰¥1 rule)
- Each rule must have: `uid`, `title`, `condition`, `data` (array)

**Template variables supported**: `__PROJECT_NAME__`, `__SHARED_PROJECT__`, `__DATASOURCE__`, `__DASHBOARD_UUID__`, table `dashboard_var` values

---

## Table Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dashboard_var` | `string` | âœ… | Variable placeholder for table name |
| `name` | `string` | âœ… | Table identifier |
| `transforms` | `Transform[]` | âœ… | Array of transformations |

### Validation Rules for Table
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique, â‰¥3 characters, start with letter, alphanumeric + underscore only
- No duplicate transform names within table

---

## SummaryTable Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | âœ… | Summary table identifier |
| `dashboard_var` | `string` | âœ… | Variable placeholder |
| `parent_table_name` | `string` | âœ… | Parent table to aggregate from |
| `sql` | `SummarySqlFile` | âœ… | SQL file configuration |

### Validation Rules
- `dashboard_var` must follow macro format, unique across summary tables
- `parent_table_name` must reference valid table from bundle
- SQL file supports template variables: `__PROJECT_NAME__`, `__SHARED_PROJECT__`, `__TABLE_NAME__`

**Example summary SQL:**
```sql
SELECT 
  toStartOfMinute(timestamp) AS minute,
  __SHARED_PROJECT___city_name(client_ip) AS city,
  COUNT(*) AS requests
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY minute, city
```

Becomes (with PROJECT_NAME=sample_project, SHARED_PROJECT=hdx_solutions, TABLE_NAME=mcdn_test):
```sql
SELECT 
  toStartOfMinute(timestamp) AS minute,
  hdx_solutions_city_name(client_ip) AS city,
  COUNT(*) AS requests
FROM sample_project.mcdn_test
GROUP BY minute, city
```

---

## Transform Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | âœ… | Relative path to transformation JSON |
| `sha256` | `string` | âŒ | Optional SHA256 hash (64 hex characters) |
| `sample` | `string` | âŒ | Deprecated (ignored) |

### Validation Rules
- Path cannot start with `/` or contain `..`, must end with `.json`
- Transform must have non-empty `name` field
- Transform must have `settings.sample_data` (non-empty object or string)
- If `subtype` exists, must equal `"firehose"`
- No duplicate transform names within table

### Transform Template Variables

Transform SQL can reference functions and dictionaries using template variables:

```json
{
  "name": "mcdn_akamai",
  "settings": {
    "sql_transform": "SELECT __SHARED_PROJECT___city_name(ip) AS city, __PROJECT_NAME___custom_parser(data) AS parsed FROM {STREAM}",
    "sample_data": {"ip": "1.2.3.4", "data": "..."}
  }
}
```

Becomes:
```sql
SELECT hdx_solutions_city_name(ip) AS city, 
       sample_project_custom_parser(data) AS parsed 
FROM {STREAM}
```

**Note**: Functions/dictionaries automatically prefixed with project name by Hydrolix, so SQL references must match deployed names.

---

## Ui, Graphics, Metadata Objects

**Ui**: `primary_url` (HTTPS), `method` (Graphics), `source` (Graphics), `data_category` (video/cdn/security)

**Graphics**: `full_title` (string), `icon_url` (HTTPS)

**Metadata**: `version` (X.Y.Z format), `maintainer` (valid email), `description` (non-empty), `channel_type` (AWS/Azure/GCP/3rdParty/Internal)

**Method-title naming**: firehose â†’ "AWS Firehose", s3 â†’ "Amazon S3", kinesis â†’ "Amazon Kinesis"

**Source-title naming**: waf source â†’ title contains "WAF"

---

## Dependencies Object

| Field | Type | Description |
|-------|------|-------------|
| `grafana` | `GrafanaDependencies` | Grafana version and plugins |
| `hydrolix` | `HydrolixDependencies` | Hydrolix resources |
| `data-sources` | `DataSource[]` | External data sources |

### HydrolixDependencies Object

| Field | Type | Description |
|-------|------|-------------|
| `cluster_version` | `string` | Required cluster version |
| `required_dictionaries` | `string[]` | Bundle-specific dictionaries (â†’ `{project}_{name}`) |
| `required_functions` | `string[]` | Bundle-specific functions (â†’ `{project}_{name}`) |
| `shared_dictionaries` | `string[]` | Shared dictionaries (â†’ `hdx_solutions_{name}`) |
| `shared_functions` | `string[]` | Shared functions (â†’ `hdx_solutions_{name}`) |

### Validation Rules
- **Shared resources** created in `hdx_solutions` project (shared across all bundles)
- **Bundle-specific resources** created in bundle's project
- **All declared resources must have local files** (deployment fails if missing)
- **Empty arrays disable auto-discovery**: `"required_functions": []` explicitly means zero bundle-specific functions
- **Omit for auto-discovery**: Don't include field to enable filesystem scanning
- Auto-discovery only triggers when BOTH `required_*` AND `shared_*` omitted for a resource type

---

## Shared Resources Architecture

### Resource Projects

Resources are deployed to one of two projects based on their declaration:

**hdx_solutions (Shared Project)**
- Contains functions and dictionaries shared across all bundles
- Resources prefixed with `hdx_solutions_` (e.g., `hdx_solutions_city_name`)
- Maintained centrally by shared resources team
- Created automatically if missing during deployment
- Reused by all bundles that declare them as shared
- Project created automatically if it doesn't exist

**Bundle Project (e.g., sample_project)**
- Contains bundle-specific resources unique to each bundle
- Resources prefixed with project name (e.g., `sample_project_custom_parser`)
- Maintained by bundle owner
- Created during bundle deployment
- Separate from other bundles

### Template Variables

**`__SHARED_PROJECT__`** - Replaced with shared project name (default: `hdx_solutions`)

Use for shared resources:
```sql
SELECT __SHARED_PROJECT___city_name(ip) AS city
FROM dictGet('__SHARED_PROJECT___geoip_city', 'city_name', ip)
```

**`__PROJECT_NAME__`** - Replaced with bundle project name (e.g., `sample_project`)

Use for bundle-specific resources:
```sql
SELECT __PROJECT_NAME___custom_parser(data) AS parsed
FROM dictGet('__PROJECT_NAME___custom_dict', 'key', value)
```

**Other template variables:**
- `__DATASOURCE__` - Grafana datasource UID
- `__DASHBOARD_UUID__` - Unique dashboard identifier
- `__TABLE_NAME__` - Table name (in dashboards and summary SQL)

### Cross-Project References

Functions and dictionaries can reference resources from any project:

```sql
-- Bundle-specific function calling shared dictionary (valid!)
CREATE FUNCTION sample_project_enrich AS
(ip) -> dictGet('hdx_solutions_geoip_city', 'city_name', ip)

-- Shared function calling another shared resource
CREATE FUNCTION hdx_solutions_country_name AS
(ip) -> dictGetString('hdx_solutions_geoip_locations', 'country', 
                      hdx_solutions_geoname_id(ip))

-- Transform using both shared and bundle-specific
SELECT 
  hdx_solutions_city_name(ip) AS city,
  sample_project_custom_parser(data) AS parsed
FROM sample_project.mcdn_test
```

---

## Function and Dictionary Files

### Function Files

**Location**: `functions/{name}.json`

**Shared function example** (`functions/city_name.json`):
```json
{
  "name": "city_name",
  "description": "Get city name from IP address",
  "sql": "(ip) -> dictGetString('__SHARED_PROJECT___geoip_city_locations_en', 'city_name', __SHARED_PROJECT___geoname_id(ip))"
}
```

**Bundle-specific function example** (`functions/custom_parser.json`):
```json
{
  "name": "custom_parser",
  "description": "Parse custom log format",
  "sql": "(data) -> dictGet('__PROJECT_NAME___custom_dict', 'value', JSONExtractString(data, 'key'))"
}
```

**Deployment:**
- Shared: Created as `hdx_solutions_city_name` in hdx_solutions project
- Bundle-specific: Created as `sample_project_custom_parser` in bundle's project

### Dictionary Files

**Location**: TWO files required per dictionary
1. Definition: `dictionaries/{name}.json`
2. Data: `dictionaries/{name}.csv` (or `.yaml`, `.yml`, `.tsv`)

**Definition example** (`dictionaries/ua_cat_dict.json`):
```json
{
  "name": "ua_cat_dict",
  "settings": {
    "filename": "ua_cat_dict",
    "format": "Regexp",
    "layout": "regexp_tree",
    "lifetime_seconds": 5,
    "output_columns": [
      {
        "name": "regexp",
        "datatype": {"type": "string", "denullify": true}
      },
      {
        "name": "ua_category",
        "datatype": {"type": "string", "denullify": true}
      }
    ],
    "primary_key": ["regexp"]
  }
}
```

**Data example** (`dictionaries/ua_cat_dict.yaml`):
```yaml
- regexp: ".*Googlebot.*"
  ua_category: "search_engine_crawler"
  is_bot: "true"
- regexp: ".*Chrome.*"
  ua_category: "browser"
  is_bot: "false"
```

**Deployment:**
- Shared: Created as `hdx_solutions_ua_cat_dict` in hdx_solutions, data file uploaded to hdx_solutions
- Bundle-specific: Created as `sample_project_ua_cat_dict` in bundle's project, data file uploaded to bundle's project

### Dictionary Zip Files

Large dictionary files can be packaged in `dictionaries/dictionaries.zip`:
- Bundle Deployer automatically extracts to `dictionaries/.extracted/`
- Extraction flattens directory structure (uses `-j` flag)
- `.extracted/` directory should be in `.gitignore`
- Files in root `dictionaries/` override extracted files (useful for custom overrides)
- Both shared and bundle-specific dictionaries can be in the zip

**Example:**
```
dictionaries/
â”œâ”€â”€ dictionaries.zip              # Large files (committed to git or Git LFS)
â”œâ”€â”€ .extracted/                   # Auto-created (gitignored)
â”‚   â”œâ”€â”€ geoip_city_blocks_ipv4.json
â”‚   â”œâ”€â”€ geoip_city_blocks_ipv4.csv
â”‚   â””â”€â”€ ...
â”œâ”€â”€ ua_cat_dict.json              # Custom override (bundle-specific)
â””â”€â”€ ua_cat_dict.yaml
```

### Auto-Discovery Mode

If `dependencies.hydrolix` is empty or omitted, the Bundle Deployer automatically:
1. Scans `functions/` for all `.json` files
2. Extracts `dictionaries.zip` (if present) to `.extracted/`
3. Scans `dictionaries/` and `.extracted/` for dictionary pairs (`.json` + data file)
4. Deploys all discovered resources as **bundle-specific** in bundle's project
5. **No shared resources are created** (must be explicitly declared)

**Explicit mode** (recommended for production):
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name", "breadcrumbs"],
      "shared_dictionaries": ["geoip_city_blocks_ipv4"],
      "required_dictionaries": ["ua_cat_dict"]
    }
  }
}
```

**Auto-discovery mode** (for vendor bundles):
```json
{
  "dependencies": {
    "hydrolix": {}
  }
}
```

**Hybrid mode** (shared explicit, bundle-specific auto-discover):
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name"]
      // Omit required_functions to auto-discover bundle-specific
    }
  }
}
```

### Auto-Discovery Trigger Logic

Auto-discovery is triggered when **both** of the following are true for a resource type:
- `required_*` field is omitted (not present in bundle.json)
- `shared_*` field is omitted (not present in bundle.json)

**Example triggering auto-discovery:**
```json
{
  "dependencies": {
    "hydrolix": {}  // No functions or dictionaries declared
  }
}
```

**Example NOT triggering auto-discovery:**
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name"]
      // required_functions omitted - NO auto-discovery (shared declared)
    }
  }
}
```

### Empty Array Behavior

**Critical**: Empty arrays explicitly declare zero resources and disable auto-discovery:

```json
{
  "required_functions": [],  // "I have zero bundle-specific functions"
  "shared_functions": ["city_name"]
}
```

This configuration:
- âœ… Creates `city_name` in `hdx_solutions`
- âŒ Does NOT auto-discover bundle-specific functions
- Result: Only shared functions exist

**To enable auto-discovery**, omit the field entirely:
```json
{
  "shared_functions": ["city_name"]
  // No required_functions field - enables auto-discovery for bundle-specific
}
```

### Validation Rules for Dependencies

#### Declared Resources (Shared or Bundle-Specific)
- âŒ **Deployment fails** if declared function missing `functions/{name}.json`
- âŒ **Deployment fails** if declared dictionary missing definition or data file
- âš ï¸ Warning if resource declared but never used in transforms
- âš ï¸ Warning if resource used in SQL but not declared

#### File Search Paths
Dictionaries searched in order:
1. `dictionaries/{name}.json` and `dictionaries/{name}.[csv/yaml/yml/tsv]`
2. `dictionaries/.extracted/{name}.json` and `dictionaries/.extracted/{name}.[csv/yaml/yml/tsv]`

First match is used, allowing root files to override extracted files.

---

## Valid Methods

- `"firehose"` - Amazon Data Firehose
- `"s3"` - Amazon S3
- `"kinesis"` - Amazon Kinesis
- `"lambda"` - AWS Lambda
- `"http_streaming"` - HTTP Streaming
- `"http"` - HTTP (alias for HTTP Streaming)

---

## Template Variable System

### Macro Variable Format

**Format**: `__VARIABLE_NAME__`

**Rules**:
- Must start and end with double underscores (`__`)
- Inner content: uppercase letters, numerals (0-9), single underscores only
- No consecutive underscores within variable name
- Minimum 5 characters total

**Valid examples**: `__PROJECT_NAME__`, `__SHARED_PROJECT__`, `__TABLE_NAME__`, `__TABLE1__`

**Invalid examples**: `_PROJECT_` (single underscores), `__project__` (lowercase), `__PROJECT__NAME__` (consecutive underscores)

### Standard Template Variables

| Variable | Used In | Replaced With | Example |
|----------|---------|---------------|---------|
| `__PROJECT_NAME__` | SQL, Dashboards | Bundle project name | `sample_project` |
| `__SHARED_PROJECT__` | SQL, Dashboards | Shared project name | `hdx_solutions` |
| `__DATASOURCE__` | Dashboards | Grafana datasource UID | `abc123def456` |
| `__DASHBOARD_UUID__` | Dashboards | Unique dashboard ID | `xyz789` |
| `__TABLE_NAME__` | Dashboards, Summary SQL | Table name | `mcdn_test` |

### Using Template Variables in SQL

**For shared resources** (in hdx_solutions project):
```sql
-- Function call
SELECT __SHARED_PROJECT___city_name(ip) AS city

-- Dictionary lookup
FROM dictGet('__SHARED_PROJECT___geoip_city', 'city_name', ip)
```

**For bundle-specific resources** (in bundle's project):
```sql
-- Function call
SELECT __PROJECT_NAME___custom_parser(data) AS parsed

-- Dictionary lookup
FROM dictGet('__PROJECT_NAME___custom_dict', 'key', value)
```

**Mixed usage** (common pattern):
```sql
-- Bundle-specific function using shared dictionary
SELECT 
  __PROJECT_NAME___enrich_data(
    ip,
    dictGet('__SHARED_PROJECT___geoip_city', 'city_name', ip)
  ) AS enriched
FROM {STREAM}
```

---

## Complete Example Bundle

```json
{
  "name": "http_streaming_mcdn_test",
  "source": "mcdn",
  "method": "http_streaming",
  "beta": true,
  "base_url": "https://github.com/hydrolix/integration-deployment-templates/blob/main/my-bundles/mcdn_test",
  "dashboard": {
    "path": "dashboards/CDN Dashboard.json",
    "project_var": "__PROJECT_NAME__"
  },
  "other_dashboards": [
    {
      "path": "dashboards/Raw Logs.json",
      "project_var": "__PROJECT_NAME__"
    }
  ],
  "alert_rules": {
    "path": "dashboards/alert-rules.json"
  },
  "dependencies": {
    "hydrolix": {
      "required_dictionaries": ["ua_cat_dict"],
      "shared_functions": ["city_name", "breadcrumbs", "country_iso_code", "geoname_id"],
      "shared_dictionaries": [
        "geoip_asn_blocks_ipv4",
        "geoip_asn_blocks_ipv6",
        "geoip_city_blocks_ipv4",
        "geoip_city_blocks_ipv6",
        "geoip_city_locations_en"
      ]
    },
    "grafana": {
      "version": "^12.1.0",
      "plugins": [
        {"name": "grafana-clickhouse-datasource", "version": "^4.10.1"}
      ]
    }
  },
  "tables": [
    {
      "dashboard_var": "__TABLE_NAME__",
      "name": "mcdn_test",
      "transforms": [
        {"path": "transformations/mcdn_akamai.json"},
        {"path": "transformations/mcdn_cloudflare.json"}
      ]
    }
  ],
  "summary_tables": [
    {
      "dashboard_var": "__SUMMARY_TABLE_NAME_1__",
      "name": "mcdn_summary_min",
      "parent_table_name": "mcdn_test",
      "sql": {"path": "summaries/mcdn_summary_min.sql"}
    },
    {
      "dashboard_var": "__SUMMARY_TABLE_NAME_2__",
      "name": "mcdn_summary_hour",
      "parent_table_name": "mcdn_test",
      "sql": {"path": "summaries/mcdn_summary_hour.sql"}
    }
  ],
  "ui": {
    "primary_url": "https://docs.hydrolix.io/docs/mcdn-integration",
    "method": {
      "full_title": "HTTP Streaming",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/http.png"
    },
    "source": {
      "full_title": "MCDN TEST",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/mcdn.png"
    },
    "data_category": "cdn"
  },
  "metadata": {
    "version": "1.0.0",
    "maintainer": "kevin.borkman@hydrolix.io",
    "description": "MCDN multi-CDN integration with shared GeoIP functions",
    "channel_type": "3rdParty"
  }
}
```

This example demonstrates:
- 4 shared functions in `hdx_solutions` project
- 5 shared dictionaries in `hdx_solutions` project
- 1 bundle-specific dictionary in bundle's project
- 2 summary tables for pre-aggregation
- Multiple transforms for different CDN formats
- Alert rules configuration
- Additional dashboards

---

## Bundle Directory Structure

```
my-bundles/mcdn_test/
â”œâ”€â”€ bundle.json                         # Bundle manifest (required)
â”œâ”€â”€ functions/                          # Custom SQL functions (optional)
â”‚   â”œâ”€â”€ city_name.json                 # Shared or bundle-specific
â”‚   â”œâ”€â”€ breadcrumbs.json
â”‚   â”œâ”€â”€ country_iso_code.json
â”‚   â””â”€â”€ geoname_id.json
â”œâ”€â”€ dictionaries/                       # Lookup tables (optional)
â”‚   â”œâ”€â”€ dictionaries.zip               # Large files (auto-extracted)
â”‚   â”œâ”€â”€ .extracted/                    # Auto-created (gitignored)
â”‚   â”‚   â”œâ”€â”€ geoip_city_blocks_ipv4.json
â”‚   â”‚   â”œâ”€â”€ geoip_city_blocks_ipv4.csv
â”‚   â”‚   â””â”€â”€ ... (other shared dictionaries from zip)
â”‚   â”œâ”€â”€ ua_cat_dict.json               # Bundle-specific
â”‚   â””â”€â”€ ua_cat_dict.yaml
â”œâ”€â”€ transformations/                    # Data parsing schemas (required)
â”‚   â”œâ”€â”€ mcdn_akamai.json
â”‚   â”œâ”€â”€ mcdn_cloudflare.json
â”‚   â””â”€â”€ mcdn_fastly.json
â”œâ”€â”€ dashboards/                         # Grafana visualizations (required)
â”‚   â”œâ”€â”€ CDN Dashboard.json             # Primary dashboard
â”‚   â”œâ”€â”€ alert-rules.json               # Alert rules (optional)
â”‚   â””â”€â”€ Raw Logs.json                  # Additional dashboards (optional)
â””â”€â”€ summaries/                          # Pre-aggregated views (optional)
    â”œâ”€â”€ mcdn_summary_min.sql
    â””â”€â”€ mcdn_summary_hour.sql
```

---

## Validation Summary

### What Gets Validated (Always)

**Structural**: Bundle JSON structure, required fields, field types, enums, URLs, paths, macro variables

**Content**: File existence, JSON syntax, dashboard structure, transforms, sample data, alert rules, summary tables

**Naming**: No duplicates, method-title consistency, source-title consistency, table name format

**Dependencies**: 
- Shared function files exist (deployment fails if missing)
- Shared dictionary files exist (deployment fails if missing)
- Bundle-specific function files exist (deployment fails if missing)
- Bundle-specific dictionary files exist (deployment fails if missing)
- SQL references match declarations (warnings for mismatches)

**Cross-Bundle**: No duplicate names, titles, tables, URLs globally

### What Gets Tested (With `--local`)

**Shared Resources**: hdx_solutions project creation, shared function/dictionary creation, idempotent reuse

**Bundle Resources**: Zip extraction, auto-discovery, bundle-specific function/dictionary creation

**Data Pipeline**: Table creation, transform deployment, sample data insertion, summary tables

**Grafana**: Datasource creation, dashboard deployment, alert rules

**Browser**: Headless Chrome testing, error detection

---

## Best Practices

### 1. Use Correct Template Variables

```sql
-- âœ… Shared resources
SELECT __SHARED_PROJECT___city_name(ip) AS city
FROM dictGet('__SHARED_PROJECT___geoip_city', 'city_name', ip)

-- âœ… Bundle-specific resources
SELECT __PROJECT_NAME___custom_parser(data) AS parsed
FROM dictGet('__PROJECT_NAME___custom_dict', 'key', value)

-- âŒ Wrong (hardcoded project names)
SELECT sample_project_city_name(ip)
SELECT reference_city_name(ip)
```

### 2. Declare Shared Resources Explicitly

For resources used by multiple bundles:
```json
{
  "shared_functions": ["city_name", "breadcrumbs"],
  "shared_dictionaries": ["geoip_city_blocks_ipv4"]
}
```

**Why**: Clear documentation, proper categorization, prevents duplicates, easier maintenance

### 3. Don't Use Empty Arrays

```json
// âŒ Empty array disables auto-discovery
{
  "required_functions": [],
  "shared_functions": ["city_name"]
}

// âœ… Omit field to enable auto-discovery
{
  "shared_functions": ["city_name"]
}
```

### 4. Include Sample Data

Every transform must have `settings.sample_data`:
```json
{
  "settings": {
    "sample_data": {
      "timestamp": 1234567890.123,
      "client_ip": "1.2.3.4"
    }
  }
}
```

### 5. Structure Dictionaries Properly

**Required**: Two files per dictionary
1. `dictionaries/{name}.json` - Definition
2. `dictionaries/{name}.csv` - Data (or `.yaml`, `.yml`, `.tsv`)

**For large files**: Package in `dictionaries.zip`, add `.extracted/` to `.gitignore`

### 6. Use Checksums for Critical Files

```json
{
  "dashboard": {
    "path": "dashboards/CDN Dashboard.json",
    "sha256": "abc123..."
  }
}
```

Generate with: `openssl dgst -sha256 file_name.json`

---

## Common Errors and Solutions

### "Transform file is not valid JSON"
**Solution**: Validate JSON syntax with `jq` or JSON linter

### "Missing required field 'name'"
**Solution**: Add `name` field to transform JSON

### "Dashboard must have __DATASOURCE__"
**Solution**: Add `"datasource": "__DATASOURCE__"` to dashboard JSON

### "Shared function 'city_name' declared but file not found"
**Solution**: Add `functions/city_name.json`, OR remove from `shared_functions`, OR check spelling

### "Auto-discovering even though shared declared"
**Cause**: Empty array in bundle.json:
```json
"required_functions": [],  // âŒ Disables auto-discovery
```

**Solution**: Remove empty array entirely

### "Unknown function hdx_solutions_city_name"
**Solution**: Ensure SQL uses `__SHARED_PROJECT__` template variable for shared functions, verify function created in hdx_solutions

### "Duplicate table name"
**Solution**: Ensure table names unique within bundle and across all bundles

---

## Getting Help

**Documentation**:
- [HOW-TO-TEST.md](./HOW-TO-TEST.md) - Testing and deployment guide
- [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) - Validation rules reference
- [Bundle Deployer.md](./Bundle%20Deployer.md) - Complete user guide
- [Hydrolix Docs](https://docs.hydrolix.io/) - Platform reference

**For validation errors**:
- Read error message carefully
- Check referenced file and line
- Review validation rules in this document
- Examine example bundles in `my-bundles/`
- Use cleanup tool: `deno run --allow-all src/cleanup.ts --all your_bundle --dry-run`