# Bundle Format Specification

A bundle is a Hydrolix JSON configuration file that packages transformations, dashboards, functions, dictionaries, and alert rules for data integration and visualization.

This document describes all valid fields and their validation rules for the TypeScript/Deno Bundle Deployer.

## Root Bundle Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Bundle identifier. Must contain only alphanumeric characters, underscores, and dashes |
| `source` | `string` | ✅ | Data source type. Must contain only alphanumeric characters, dashes, and underscores |
| `method` | `string` | ✅ | Integration method. See [Valid Methods](#valid-methods) |
| `beta` | `boolean` | ✅ | Whether this is a beta release |
| `base_url` | `string` | ✅ | HTTPS URL to the repository base path |
| `dashboard` | `Dashboard` | ✅ | Primary dashboard configuration |
| `other_dashboards` | `Dashboard[]` | ❌ | Optional additional dashboard configurations |
| `alert_rules` | `AlertRules` | ❌ | Optional alert rules configuration |
| `tables` | `Table[]` | ✅ | Array of table definitions |
| `summary_tables` | `SummaryTable[]` | ❌ | Optional array of summary table definitions |
| `ui` | `Ui` | ✅ | User interface configuration |
| `metadata` | `Metadata` | ✅ | Bundle metadata |
| `method_overrides` | `MethodOverrides` | ❌ | Optional method-specific overrides |
| `dependencies` | `Dependencies` | ❌ | Optional dependency requirements |

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
| `path` | `string` | ✅ | Relative path to dashboard JSON file |
| `project_var` | `string` | ✅ | Variable placeholder for project name (must be `__PROJECT_NAME__`) |
| `sha256` | `string` | ❌ | Optional SHA256 hash of dashboard contents (64 hex characters) |

### Validation Rules for Dashboard
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json`
- `project_var` must be `__PROJECT_NAME__`
- Dashboard JSON must contain all required template variables:
  - `__DASHBOARD_UUID__`
  - `__DATASOURCE__`
  - `__PROJECT_NAME__`
  - All table `dashboard_var` values
  - All summary table `dashboard_var` values (if defined)
- Dashboard must have top-level `dashboard` object
- Dashboard must not have hardcoded `id` field
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

### Required Dashboard Template Variables

All dashboards must include these template variables:

```json
{
  "dashboard": {
    "id": null,  // Must be null
    "uid": "__DASHBOARD_UUID__",  // Required
    "title": "My Dashboard",
    // ...
  },
  "datasource": "__DATASOURCE__",  // Required
  // References to tables using dashboard_var:
  "table": "__TABLE_NAME__"  // Must match table.dashboard_var
}
```

---

## Alert Rules Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to alert rules JSON file |
| `sha256` | `string` | ❌ | Optional SHA256 hash of alert rules contents (64 hex characters) |

### Validation Rules for Alert Rules
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json`
- Alert rules JSON must be valid Grafana alert rules format
- Must contain `apiVersion` field
- Must contain `groups` array with at least one group
- Each group must have: `name`, `folder`, `interval`, `rules`
- Each rule must have: `uid`, `title`, `condition`, `data`
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

### Alert Rules Template Variables

Alert rules support the same template variables as dashboards:
- `__PROJECT_NAME__` - Replaced with project name
- `__DATASOURCE__` - Replaced with Grafana datasource UID
- `__DASHBOARD_UUID__` - Replaced with dashboard UID (for linking)
- Table `dashboard_var` values - Replaced with full table names

### Example Alert Rules Structure

```json
{
  "apiVersion": 1,
  "groups": [
    {
      "name": "Traffic Monitoring",
      "folder": "CDN Alerts",
      "interval": "5m",
      "rules": [
        {
          "uid": "alert_high_errors",
          "title": "High Error Rate",
          "condition": "C",
          "data": [
            {
              "refId": "A",
              "datasourceUid": "__DATASOURCE__",
              "model": {
                "query": "SELECT COUNT(*) FROM __PROJECT_NAME__.__TABLE_NAME__ WHERE status >= 500"
              }
            }
          ],
          "for": "5m",
          "annotations": {
            "description": "Error rate exceeded threshold"
          }
        }
      ]
    }
  ]
}
```

---

## Table Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dashboard_var` | `string` | ✅ | Variable placeholder for table name in dashboard |
| `name` | `string` | ✅ | Table identifier |
| `transforms` | `Transform[]` | ✅ | Array of transformation definitions |

### Validation Rules for Table
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique across all tables in the bundle
- `name` must be ≥ 3 characters
- `name` must start with a letter (a-z, A-Z)
- `name` must contain only alphanumeric characters and underscores
- No duplicate transform names within a table

---

## SummaryTable Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Summary table identifier |
| `dashboard_var` | `string` | ✅ | Variable placeholder for summary table name in dashboard |
| `parent_table_name` | `string` | ✅ | Name of the parent table to aggregate from |
| `sql` | `SummarySqlFile` | ✅ | SQL file configuration for summary table |

### Validation Rules for SummaryTable
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `dashboard_var` must be unique across all summary tables in the bundle
- `name` must be unique across all summary tables in the bundle
- `parent_table_name` must reference a valid table name from the bundle's `tables` array

### Summary Table Template Variables

Summary SQL files support these template variables:
- `__PROJECT_NAME__` - Replaced with project name
- `__TABLE_NAME__` - Replaced with `parent_table_name`

### Example Summary SQL

```sql
-- summaries/mcdn_summary_min.sql
SELECT 
  toStartOfMinute(timestamp) AS minute,
  cdn_provider,
  COUNT(*) AS requests,
  SUM(bytes_sent) AS total_bytes,
  AVG(response_time) AS avg_response_time
FROM __PROJECT_NAME__.__TABLE_NAME__
WHERE timestamp >= now() - INTERVAL 1 HOUR
GROUP BY minute, cdn_provider
```

When deployed with:
- `__PROJECT_NAME__` = `sample_project`
- `__TABLE_NAME__` = `mcdn_test` (from `parent_table_name`)

Becomes:
```sql
SELECT 
  toStartOfMinute(timestamp) AS minute,
  cdn_provider,
  COUNT(*) AS requests,
  SUM(bytes_sent) AS total_bytes,
  AVG(response_time) AS avg_response_time
FROM sample_project.mcdn_test
WHERE timestamp >= now() - INTERVAL 1 HOUR
GROUP BY minute, cdn_provider
```

---

## SummarySqlFile Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to SQL file |
| `sha256` | `string` | ❌ | Optional SHA256 hash of SQL file contents (64 hex characters) |

### Validation Rules for SummarySqlFile
- Path cannot start with `/`
- Path cannot contain `..`
- Path must end with `.sql`
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

---

## Transform Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to transformation JSON |
| `sha256` | `string` | ❌ | Optional SHA256 hash of transformation contents (64 hex characters) |
| `sample` | `string` | ❌ | Deprecated field (ignored) |

### Validation Rules for Transform
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json`
- Transform JSON must be valid
- Transform must have non-empty `name` field
- Transform must have `settings.sample_data` (non-empty object or string)
- If `subtype` field exists, must equal `"firehose"`
- No duplicate transform names within same table
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

### Transform Template Variables

Transform SQL can reference custom functions and dictionaries:
- `__PROJECT_NAME___function_name()` - Custom function call
- `dictGet('__PROJECT_NAME___dict_name', 'column', key)` - Dictionary lookup

### Example Transform with Template Variables

```json
{
  "name": "mcdn_akamai_ds2",
  "type": "csv",
  "settings": {
    "sql_transform": "SELECT \n  toUInt64(timestamp * 1000) AS timestamp,\n  __PROJECT_NAME___city_name(client_ip) AS city,\n  dictGet('__PROJECT_NAME___ua_cat_dict', 'category', user_agent) AS ua_category\nFROM {STREAM}",
    "sample_data": {
      "timestamp": 1234567890.123,
      "client_ip": "1.2.3.4",
      "user_agent": "Mozilla/5.0..."
    }
  }
}
```

When deployed with `__PROJECT_NAME__` = `sample_project`, becomes:

```sql
SELECT 
  toUInt64(timestamp * 1000) AS timestamp,
  sample_project_city_name(client_ip) AS city,
  dictGet('sample_project_ua_cat_dict', 'category', user_agent) AS ua_category
FROM {STREAM}
```

**Note**: Functions and dictionaries are automatically prefixed with project name by Hydrolix, so references must match:
- Function deployed as: `sample_project_city_name`
- Dictionary deployed as: `sample_project_ua_cat_dict`
- SQL must reference with prefix: `sample_project_city_name()`, `dictGet('sample_project_ua_cat_dict', ...)`

---

## Ui Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `primary_url` | `string` | ✅ | HTTPS URL to primary documentation |
| `method` | `Graphics` | ✅ | Method display information |
| `source` | `Graphics` | ✅ | Source display information |
| `data_category` | `string` | ✅ | Data category classification |

### Validation Rules for Ui
- `primary_url` must start with `https://` or `file://`
- `data_category` must be one of: `"video"`, `"cdn"`, `"security"`
- `method.full_title` should align with `method` field (see naming conventions below)
- `source.full_title` must be unique across all bundles

### Method-Title Naming Conventions

| Method | Expected UI Title Contains |
|--------|---------------------------|
| `firehose` | "Amazon Data Firehose", "AWS Firehose", or "Kinesis Data Firehose" |
| `s3` | "Amazon S3" or "AWS S3" |
| `kinesis` | "Amazon Kinesis" or "AWS Kinesis" |
| `http_streaming` | No specific requirement |
| `http` | No specific requirement |

### Source-Title Naming Conventions

- If `source` is `"waf"`, the `source.full_title` must contain "WAF" (case-insensitive)

---

## Graphics Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `full_title` | `string` | ✅ | Display title for the component |
| `icon_url` | `string` | ✅ | HTTPS URL to icon image |

### Validation Rules for Graphics
- `icon_url` must start with `https://` or `file://`

---

## Metadata Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `string` | ✅ | Semantic version number |
| `maintainer` | `string` | ✅ | Maintainer email address |
| `description` | `string` | ✅ | Bundle description |
| `channel_type` | `string` | ✅ | Distribution channel type |

### Validation Rules for Metadata
- `version` must follow semantic versioning format (e.g., `1.0.0`)
- `version` must have exactly two dots (X.Y.Z)
- `maintainer` must be a valid email address (contain `@` and `.`)
- `description` cannot be empty or whitespace only
- `channel_type` must be one of: `"AWS"`, `"Azure"`, `"GCP"`, `"3rdParty"`, `"Internal"`

---

## MethodOverrides Object (Optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `region` | `string` | ❌  | AWS region override |
| `stream_prefix` | `string` | ❌  | Stream name prefix |

### Validation Rules for MethodOverrides
- No specific validation constraints beyond basic string format

---

## Dependencies Object (Optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `grafana` | `GrafanaDependencies` | ❌ | Grafana version and plugin requirements |
| `hydrolix` | `HydrolixDependencies` | ❌ | Hydrolix cluster requirements |
| `data-sources` | `DataSource[]` | ❌ | External data source configurations |

### HydrolixDependencies Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `cluster_version` | `string` | ❌ | Required Hydrolix cluster version |
| `required_dictionaries` | `string[]` | ❌ | List of required dictionary names |
| `required_functions` | `string[]` | ❌ | List of required function names |

### Function and Dictionary Files

**Functions** are defined in `functions/{name}.json`:

```json
{
  "name": "city_name",
  "description": "Get city name from IP address",
  "sql": "(ip) -> dictGetString('__PROJECT_NAME___geoip_city_locations_en', 'city_name', __PROJECT_NAME___geoname_id(ip))"
}
```

**Dictionaries** require two files:
1. Definition: `dictionaries/{name}.json`
2. Data: `dictionaries/{name}.csv` (or `.yaml`, `.yml`, `.tsv`)

**Dictionary definition example** (`dictionaries/ua_cat_dict.json`):
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

**Dictionary data example** (`dictionaries/ua_cat_dict.yaml`):
```yaml
- regexp: ".*Googlebot.*"
  ua_category: "search_engine_crawler"
  is_bot: "true"
- regexp: ".*Chrome.*"
  ua_category: "browser"
  is_bot: "false"
```

### Dictionary Zip Files

Large dictionary files can be packaged in `dictionaries/dictionaries.zip`:
- Bundle Deployer automatically extracts to `dictionaries/.extracted/`
- Extraction uses `-j` flag to flatten directory structure
- `.extracted/` directory should be in `.gitignore`
- Files in root `dictionaries/` override extracted files

### Auto-Discovery Mode

If `dependencies.hydrolix` is empty or omitted, the Bundle Deployer automatically:
1. Scans `functions/` for all `.json` files
2. Extracts `dictionaries.zip` (if present)
3. Scans `dictionaries/` and `.extracted/` for dictionary pairs (`.json` + data file)
4. Deploys all discovered resources

**Explicit mode** (recommended):
```json
{
  "dependencies": {
    "hydrolix": {
      "required_functions": ["city_name", "breadcrumbs"],
      "required_dictionaries": ["ua_cat_dict", "geoip_city_blocks_ipv4"]
    }
  }
}
```

**Auto-discovery mode** (zero config):
```json
{
  "dependencies": {
    "hydrolix": {}
  }
}
```

### Validation Rules for Dependencies

#### Function Dependencies
- ⚠️ Warning if function declared but `functions/{name}.json` missing
- ⚠️ Warning if function used in transform SQL but not declared
- ⚠️ Warning if function declared but never used in transforms

#### Dictionary Dependencies
- ⚠️ Warning if dictionary declared but `dictionaries/{name}.json` missing
- ⚠️ Warning if dictionary declared but data file missing
- ⚠️ Warning if dictionary used in transform SQL (`dictGet`, `dictGetString`) but not declared
- ⚠️ Warning if dictionary declared but never used in transforms

#### Production Mode (`--production` flag)
- ❌ Error if declared function doesn't exist on cluster (with project prefix)
- ❌ Error if declared dictionary doesn't exist on cluster (with project prefix)
- ⚠️ Warning if local definition files missing

---

## Valid Methods

The `method` field must be one of:
- `"firehose"` - Amazon Data Firehose
- `"s3"` - Amazon S3
- `"kinesis"` - Amazon Kinesis
- `"lambda"` - AWS Lambda
- `"http_streaming"` - HTTP Streaming
- `"http"` - HTTP (alias for HTTP Streaming)

---

## Template Variable System

### Macro Variable Format

Several fields use macro variable format for template substitution:

**Format**: `__VARIABLE_NAME__`

**Rules**:
- Must start and end with double underscores (`__`)
- Inner content must be uppercase letters, numerals (0-9), and single underscores only
- No consecutive underscores within the variable name
- Minimum 5 characters total (e.g., `__X__`)
- Inner content cannot be empty

**Examples**:
- ✅ `__PROJECT_NAME__`
- ✅ `__TABLE_NAME__`
- ✅ `__DATA_SOURCE__`
- ✅ `__TABLE1__`
- ✅ `__SUMMARY_TABLE_NAME_1__`
- ❌ `_PROJECT_` (single underscores)
- ❌ `__project_name__` (lowercase)
- ❌ `__PROJECT__NAME__` (consecutive underscores)
- ❌ `____` (empty inner content)

### Standard Template Variables

| Variable | Used In | Replaced With | Example |
|----------|---------|---------------|---------|
| `__PROJECT_NAME__` | Everywhere | Project name | `sample_project` |
| `__DATASOURCE__` | Dashboards | Grafana datasource UID | `abc123def456` |
| `__DASHBOARD_UUID__` | Dashboards | Unique dashboard ID | `xyz789` |
| `__TABLE_NAME__` | Dashboards | Table name | `mcdn_test` |
| `__SUMMARY_TABLE_NAME_1__` | Dashboards | Summary table name | `mcdn_summary_min` |

### Using `__PROJECT_NAME__` in SQL

**Critical**: Hydrolix automatically prefixes functions and dictionaries with the project name. Always use `__PROJECT_NAME__` to ensure references match deployed names.

**In function definitions** (`functions/city_name.json`):
```json
{
  "sql": "(ip) -> dictGetString('__PROJECT_NAME___geoip_dict', 'city', ip)"
}
```

**In transform SQL**:
```sql
SELECT 
  __PROJECT_NAME___city_name(client_ip) AS city,
  dictGet('__PROJECT_NAME___ua_cat_dict', 'category', user_agent) AS category
FROM {STREAM}
```

**After deployment** (with `__PROJECT_NAME__` = `sample_project`):
```sql
SELECT 
  sample_project_city_name(client_ip) AS city,
  dictGet('sample_project_ua_cat_dict', 'category', user_agent) AS category
FROM sample_project.mcdn_test
```

**Deployed resource names**:
- Function: `sample_project_city_name` (Hydrolix adds prefix)
- Dictionary: `sample_project_ua_cat_dict` (Hydrolix adds prefix)
- Table: `sample_project.mcdn_test` (full qualified name)

### Dashboard Variable Replacement

**In dashboard JSON**:
```json
{
  "dashboard": {
    "uid": "__DASHBOARD_UUID__",
    "title": "CDN Dashboard"
  },
  "datasource": {
    "uid": "__DATASOURCE__"
  },
  "panels": [
    {
      "targets": [
        {
          "query": "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__"
        }
      ]
    }
  ]
}
```

**After deployment**:
```json
{
  "dashboard": {
    "uid": "xyz789abc",
    "title": "CDN Dashboard"
  },
  "datasource": {
    "uid": "abc123def456"
  },
  "panels": [
    {
      "targets": [
        {
          "query": "SELECT * FROM sample_project.mcdn_test"
        }
      ]
    }
  ]
}
```

---

## URL Validation

HTTPS URLs must:
- Start with `https://` or `file://`
- Be valid URLs according to URL parsing standards

Path fields must:
- Not start with `/`
- Not contain `..` (directory traversal)
- End with appropriate extension:
  - `.json` for dashboards, transforms, functions, dictionaries, alert rules
  - `.sql` for summary table SQL files
  - `.csv`, `.yaml`, `.yml`, or `.tsv` for dictionary data files

---

## SHA256 Hash Format

SHA256 hash fields must:
- Be exactly 64 characters long
- Contain only hexadecimal characters (0-9, a-f, A-F)

Generate checksums with:
```bash
openssl dgst -sha256 file_name.json
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
    "project_var": "__PROJECT_NAME__",
    "sha256": "abc123..."
  },
  "other_dashboards": [
    {
      "path": "dashboards/Raw Logs.json",
      "project_var": "__PROJECT_NAME__"
    }
  ],
  "alert_rules": {
    "path": "dashboards/alert-rules.json",
    "sha256": "def456..."
  },
  "dependencies": {
    "hydrolix": {
      "required_functions": ["city_name", "breadcrumbs"],
      "required_dictionaries": ["ua_cat_dict", "geoip_city_blocks_ipv4"]
    }
  },
  "tables": [
    {
      "dashboard_var": "__TABLE_NAME__",
      "name": "mcdn_test",
      "transforms": [
        {
          "path": "transformations/mcdn_akamai_ds2.json",
          "sha256": "789abc..."
        },
        {
          "path": "transformations/mcdn_cloudflare.json"
        }
      ]
    }
  ],
  "summary_tables": [
    {
      "dashboard_var": "__SUMMARY_TABLE_NAME_1__",
      "name": "mcdn_summary_min",
      "parent_table_name": "mcdn_test",
      "sql": {
        "path": "summaries/mcdn_summary_min.sql",
        "sha256": "012def..."
      }
    },
    {
      "dashboard_var": "__SUMMARY_TABLE_NAME_2__",
      "name": "mcdn_summary_hour",
      "parent_table_name": "mcdn_test",
      "sql": {
        "path": "summaries/mcdn_summary_hour.sql"
      }
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
    "maintainer": "user@example.com",
    "description": "MCDN multi-CDN integration with functions and dictionaries",
    "channel_type": "3rdParty"
  }
}
```

---

## Bundle Directory Structure

```
my-bundles/mcdn_test/
├── bundle.json                         # Bundle manifest (required)
├── functions/                          # Custom SQL functions (optional)
│   ├── city_name.json
│   └── breadcrumbs.json
├── dictionaries/                       # Lookup tables (optional)
│   ├── dictionaries.zip               # Large files (auto-extracted)
│   ├── .extracted/                    # Auto-created (gitignored)
│   ├── ua_cat_dict.json               # Dictionary definition
│   ├── ua_cat_dict.yaml               # Dictionary data
│   ├── geoip_city_blocks_ipv4.json
│   └── geoip_city_blocks_ipv4.csv
├── transformations/                    # Data parsing schemas (required)
│   ├── mcdn_akamai_ds2.json
│   ├── mcdn_cloudflare.json
│   └── mcdn_fastly.json
├── dashboards/                         # Grafana visualizations (required)
│   ├── CDN Dashboard.json             # Primary dashboard
│   ├── alert-rules.json               # Alert rules (optional)
│   └── Raw Logs.json                  # Additional dashboards (optional)
└── summaries/                          # Pre-aggregated views (optional)
    ├── mcdn_summary_min.sql
    └── mcdn_summary_hour.sql
```

---

## Validation Summary

### Structural Validation
✅ Bundle JSON structure  
✅ Required fields present  
✅ Field types correct  
✅ Enum values valid  
✅ URL formats valid  
✅ Path formats valid  
✅ Macro variable formats valid

### Content Validation
✅ File existence  
✅ JSON syntax  
✅ Dashboard structure  
✅ Transform structure  
✅ Sample data presence  
✅ Alert rules structure (if present)  
✅ Summary table SQL (if present)

### Naming Validation
✅ No duplicate table names  
✅ No duplicate dashboard variables  
✅ Method-title consistency  
✅ Source-title consistency  
✅ Bundle name includes source and method  
✅ Table names start with letter  
✅ Table names are alphanumeric + underscore only

### Dependency Validation
✅ Function files exist (warning if missing)  
✅ Dictionary files exist (warning if missing)  
✅ SQL references match declarations (warning if mismatch)  
✅ No unused declarations (warning)

### Cross-Bundle Validation
✅ No duplicate bundle names (global)  
✅ No duplicate UI source titles (global)  
✅ No duplicate table names (global)  
✅ No duplicate base URLs (global)

### Integration Testing (with `--local`)
✅ Dictionary zip extraction  
✅ Function creation in Hydrolix  
✅ Dictionary creation in Hydrolix  
✅ Table creation  
✅ Transform deployment  
✅ Sample data insertion  
✅ Summary table creation  
✅ Grafana datasource creation  
✅ Dashboard deployment  
✅ Alert rules deployment  
✅ Headless browser testing

---

## Best Practices

### 1. Always Use Template Variables

```sql
-- ✅ Correct
SELECT __PROJECT_NAME___city_name(ip) AS city
FROM __PROJECT_NAME__.__TABLE_NAME__

-- ❌ Wrong (hardcoded)
SELECT sample_project_city_name(ip) AS city
FROM sample_project.mcdn_test
```

### 2. Include Sample Data

Every transform must have `settings.sample_data`:
```json
{
  "settings": {
    "sample_data": {
      "timestamp": 1234567890.123,
      "client_ip": "1.2.3.4",
      "user_agent": "Mozilla/5.0..."
    }
  }
}
```

### 3. Declare Dependencies

**Explicit mode** for production bundles:
```json
{
  "dependencies": {
    "hydrolix": {
      "required_functions": ["city_name"],
      "required_dictionaries": ["ua_cat_dict"]
    }
  }
}
```

**Auto-discovery mode** for vendor bundles:
```json
{
  "dependencies": {
    "hydrolix": {}
  }
}
```

### 4. Use Checksums for Critical Files

```json
{
  "dashboard": {
    "path": "dashboards/CDN Dashboard.json",
    "sha256": "abc123..."
  }
}
```

Generate with: `openssl dgst -sha256 file_name.json`

### 5. Structure Dictionaries Properly

**Two files required**:
1. `dictionaries/{name}.json` - Definition
2. `dictionaries/{name}.csv` - Data (or `.yaml`, `.yml`, `.tsv`)

**Large dictionaries**: Package in `dictionaries.zip` and add `.extracted/` to `.gitignore`

### 6. Test Incrementally

```bash
# 1. Validate structure
deno run --allow-all src/main.ts mcdn_test

# 2. Test dashboard only
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# 3. Full integration test
deno run --allow-all src/main.ts --local mcdn_test
```

---

## Common Errors and Solutions

### "Transform file is not valid JSON"
**Solution**: Validate JSON syntax with `jq` or JSON linter

### "Missing required field 'name'"
**Solution**: Add `name` field to transform JSON

### "Dashboard must have __DATASOURCE__"
**Solution**: Add `"datasource": "__DATASOURCE__"` to dashboard JSON

### "Unknown function sample_project_city_name"
**Solution**: Ensure function SQL uses `__PROJECT_NAME__` and function was created successfully

### "Duplicate table name"
**Solution**: Ensure all table names are unique within bundle

### "Base URL doesn't match expected format"
**Solution**: Update `base_url` to match GitHub repository path

---

## Getting Help

**Documentation**:
- [HOW-TO-TEST.md](./HOW-TO-TEST.md) - Testing and deployment guide
- [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) - Validation rules reference
- [Bundle Deployer.md](./Bundle%20Deployer.md) - Complete user guide
- [Hydrolix Docs](https://docs.hydrolix.io/) - Platform reference

**Validation Errors**:
- Read error message carefully
- Check referenced file and line
- Review this specification
- Examine example bundles in `my-bundles/`
- Use cleanup tool for fresh starts