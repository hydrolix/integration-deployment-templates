# Bundle Format Documentation v2

A bundle is a Hydrolix JSON configuration file that packages transformations, dashboards, and documentation for data integration and visualization.

This document describes all valid fields and their validation rules.

## Root Bundle Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Bundle identifier. Must contain only alphanumeric characters, underscores, and dashes |
| `source` | `string` | ✅ | Data source type. Must contain only alphanumeric characters, dashes, and underscores |
| `method` | `string` | ✅ | Integration method. See [Valid Methods](#valid-methods) |
| `beta` | `boolean` | ✅ | Whether this is a beta release |
| `base_url` | `string` | ✅ | HTTPS URL to the repository base path |
| `dashboard` | `Dashboard` | ✅ | Dashboard configuration |
| `other_dashboards` | `Dashboard[]` | ❌ | Optional additional dashboard configurations |
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
- **Dependency validation**: All resources referenced in dependency fields must exist (see [Dependency Validation](#dependency-validation))
- **DAG validation**: The dependency graph must be acyclic (no circular dependencies)

## Dashboard Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to dashboard JSON or TSV file |
| `project_var` | `string` | ✅ | Variable placeholder for project name |
| `sha256` | `string` | ❌ | Optional SHA256 hash of dashboard contents (64 hex characters) |
| `requires_plugins` | `string[]` | ❌ | Optional array of Grafana plugin names required by this dashboard |

### Validation Rules for Dashboard
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json` or `.tsv`
- `project_var` must follow macro format: `__VARIABLE_NAME__`
- Use `openssl dgst -sha256 <file_name>` to generate the sha256
- All plugin names in `requires_plugins` must exist in `dependencies.grafana.plugins[].name`

## Table Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dashboard_var` | `string` | ✅ | Variable placeholder for table name in dashboard |
| `name` | `string` | ✅ | Table identifier |
| `transforms` | `Transform[]` | ✅ | Array of transformation definitions |
| `requires_functions` | `string[]` | ❌ | **NEW**: Optional array of custom function names this table depends on |
| `requires_dictionaries` | `string[]` | ❌ | **NEW**: Optional array of dictionary names this table depends on directly |

### Validation Rules for Table
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique across all tables in the bundle
- All names in `requires_functions` must exist in `dependencies.hydrolix.required_functions[].name`
- All names in `requires_dictionaries` must exist in `dependencies.hydrolix.required_dictionaries[].name`

## SummaryTable Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Summary table identifier |
| `dashboard_var` | `string` | ✅ | Variable placeholder for summary table name in dashboard |
| `parent_table_name` | `string` | ✅ | Name of the parent table to aggregate from |
| `sql` | `SummarySqlFile` | ✅ | SQL file configuration for summary table |
| `requires_functions` | `string[]` | ❌ | **NEW**: Optional array of custom function names this summary table depends on |
| `requires_dictionaries` | `string[]` | ❌ | **NEW**: Optional array of dictionary names this summary table depends on directly |

### Validation Rules for SummaryTable
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `dashboard_var` must be unique across all summary tables in the bundle
- `name` must be unique across all summary tables in the bundle
- `parent_table_name` must reference a valid table name from the bundle's `tables` array
- All names in `requires_functions` must exist in `dependencies.hydrolix.required_functions[].name`
- All names in `requires_dictionaries` must exist in `dependencies.hydrolix.required_dictionaries[].name`

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

## Transform Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to transformation JSON |
| `sha256` | `string` | ❌ | Optional SHA256 hash of transformation contents (64 hex characters) |
| `sample` | `string` | ❌ | Optional relative path to sample data file |

### Validation Rules for Transform
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json`
- `sample` cannot start with `/`
- `sample` cannot contain `..`
- `sample` must end with `.json` or `.tsv`
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

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

## Graphics Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `full_title` | `string` | ✅ | Display title for the component |
| `icon_url` | `string` | ✅ | HTTPS URL to icon image |

### Validation Rules for Graphics
- `icon_url` must start with `https://` or `file://`

## Metadata Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `string` | ✅ | Semantic version number |
| `maintainer` | `string` | ✅ | Maintainer email address |
| `description` | `string` | ✅ | Bundle description |
| `channel_type` | `string` | ✅ | Distribution channel type |

### Validation Rules for Metadata
- `version` must follow semantic versioning format (e.g., `1.0.0`)
- `maintainer` must be a valid email address (contain `@` and `.`)
- `description` cannot be empty or whitespace only
- `channel_type` must be one of: `"AWS"`, `"Azure"`, `"GCP"`, `"3rdParty"`, `"Internal"`

## MethodOverrides Object (Optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `region` | `string` | ❌  | AWS region override |
| `stream_prefix` | `string` | ❌  | Stream name prefix |

### Validation Rules for MethodOverrides
- No specific validation constraints beyond basic string format

## Dependencies Object (Optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `grafana` | `GrafanaDependencies` | ❌ | Grafana version and plugin requirements |
| `hydrolix` | `HydrolixDependencies` | ❌ | Hydrolix cluster requirements |
| `data-sources` | `DataSource[]` | ❌ | External data source configurations |

### GrafanaDependencies Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `string` | ❌ | Required Grafana version (semver range) |
| `plugins` | `GrafanaPlugin[]` | ❌ | Required Grafana plugins |

### GrafanaPlugin Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅  | Plugin identifier (e.g., `grafana-clickhouse-datasource`, `marcusolsson-treemap-panel`) |
| `version` | `string` | ✅  | Plugin version requirement (semver range) |
| `type` | `string` | ❌  | Plugin type: `"datasource"` or `"panel"` (optional, for documentation) |

### HydrolixDependencies Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `cluster_version` | `string` | ❌ | Required Hydrolix cluster version |
| `required_dictionaries` | `Dictionary[]` | ❌ | Required external dictionaries |
| `required_functions` | `Function[]` | ❌ | Required custom functions |

### Dictionary Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅  | Dictionary identifier |
| `source` | `string` | ✅  | HTTPS URL to dictionary source |

### Function Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅  | Function name |
| `definition` | `string` | ✅  | Function SQL definition |
| `requires_dictionaries` | `string[]` | ❌ | **NEW**: Optional array of dictionary names this function depends on |

### Validation Rules for Function
- All names in `requires_dictionaries` must exist in `dependencies.hydrolix.required_dictionaries[].name`

### DataSource Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅  | Data source name |
| `type` | `string` | ✅  | Data source type |
| `url` | `string` | ✅  | Data source connection URL |
| `access` | `string` | ✅  | Data access mode |

### Validation Rules for Dependencies
- All URLs in `dependencies.hydrolix.required_dictionaries[].source` must start with `https://` or `file://`
- `dependencies.grafana.plugins[].name` must be valid plugin identifiers
- `dependencies.grafana.version` must follow semantic version range format
- `dependencies.hydrolix.cluster_version` must follow semantic version range format

## Dependency Validation

The bundle format now supports explicit dependency tracking to ensure resources are created in the correct order. The dependency graph must be validated to ensure:

1. **Reference Integrity**: All referenced resources exist
2. **Acyclic Graph**: No circular dependencies exist
3. **Execution Order**: A valid topological sort can be computed

### Dependency Chain

The typical dependency chain is:

```
Dictionaries
    ↓
Functions (may depend on dictionaries)
    ↓
Tables (may depend on functions and/or dictionaries)
    ↓
Summary Tables (depend on parent tables, may depend on functions and/or dictionaries)
```

### Validation Rules

1. **Dictionary References**:
   - `dependencies.hydrolix.required_functions[].requires_dictionaries[]` names must exist in `dependencies.hydrolix.required_dictionaries[].name`
   - `tables[].requires_dictionaries[]` names must exist in `dependencies.hydrolix.required_dictionaries[].name`
   - `summary_tables[].requires_dictionaries[]` names must exist in `dependencies.hydrolix.required_dictionaries[].name`

2. **Function References**:
   - `tables[].requires_functions[]` names must exist in `dependencies.hydrolix.required_functions[].name`
   - `summary_tables[].requires_functions[]` names must exist in `dependencies.hydrolix.required_functions[].name`

3. **Table References**:
   - `summary_tables[].parent_table_name` must exist in `tables[].name`

4. **Plugin References**:
   - `dashboard.requires_plugins[]` names must exist in `dependencies.grafana.plugins[].name`
   - `other_dashboards[].requires_plugins[]` names must exist in `dependencies.grafana.plugins[].name`

5. **Acyclic Validation**:
   - The complete dependency graph must not contain cycles
   - Use topological sort to validate and compute execution order

### Execution Order Computation

To compute the correct execution order:

1. Build a directed graph where:
   - Each dictionary, function, table, and summary table is a node
   - Each dependency relationship is an edge

2. Perform topological sort to get execution order:
   ```
   Level 0: Dictionaries (no dependencies)
   Level 1: Functions that depend only on dictionaries
   Level 2: Functions that depend on Level 1 functions
   Level 3: Tables that depend on dictionaries/functions
   Level 4: Summary tables that depend on tables
   ...and so on
   ```

3. Resources at the same level can be created in parallel

## Valid Methods

The `method` field must be one of:
- `"firehose"` - Amazon Data Firehose
- `"s3"` - Amazon S3
- `"kinesis"` - Amazon Kinesis
- `"lambda"` - AWS Lambda
- `"http_streaming"` - HTTP Streaming
- `"http"` - Alias for HTTP Streaming

## Macro Variable Format

Several fields use macro variable format for template substitution:

**Format**: `__VARIABLE_NAME__`

**Rules**:
- Must start and end with double underscores (`__`)
- Inner content must be uppercase letters, numerals (1-9), and single underscores only
- No consecutive underscores within the variable name
- Minimum 5 characters total (e.g., `__X__`)
- Inner content cannot be empty

**Examples**:
- ✅ `__PROJECT_NAME__`
- ✅ `__TABLE_NAME__`
- ✅ `__DATA_SOURCE__`
- ✅ `__TABLE1__`
- ❌ `_PROJECT_` (single underscores)
- ❌ `__project_name__` (lowercase)
- ❌ `__PROJECT__NAME__` (consecutive underscores)
- ❌ `____` (empty inner content)

## URL Validation

HTTPS URLs must:
- Start with `https://` or `file://`
- Be valid URLs according to URL parsing standards

Path fields must:
- Not start with `/`
- Not contain `..` (directory traversal)
- End with appropriate extension:
  - `.json` or `.tsv` for dashboards and samples
  - `.json` for transformations
  - `.sql` for summary table SQL files

## SHA256 Hash Format

SHA256 hash fields must:
- Be exactly 64 characters long
- Contain only hexadecimal characters (0-9, a-f, A-F)

## Example Bundle with Complete Dependency Chain

```json
{
  "name": "kinesis-cloudfront",
  "source": "cloudfront",
  "method": "kinesis",
  "beta": true,
  "base_url": "https://github.com/hydrolix/integration-deployment-templates/automation/cloudfront-to-kinesis",
  "dashboard": {
    "path": "dashboards/current.json",
    "project_var": "__PROJECT_NAME__",
    "sha256": "65d22b569bb986a28e98246637bd41dad5ecf56220965d2cc3491577a160138b",
    "requires_plugins": ["marcusolsson-treemap-panel"]
  },
  "other_dashboards": [
    {
      "path": "dashboards/alternate.json",
      "project_var": "__PROJECT_NAME__"
    }
  ],
  "tables": [
    {
      "dashboard_var": "__TABLE_NAME__",
      "name": "cloudfront_kinesis",
      "requires_functions": ["lookup_city", "lookup_asn"],
      "transforms": [
        {
          "path": "transformations/current.json",
          "sha256": "88cb72324adb0c77e657a883552f086bc014985f0c4738ea84ad976a403dc3ac",
          "sample": "samples/cloudfront_sample.json"
        }
      ]
    }
  ],
  "summary_tables": [
    {
      "name": "cloudfront_hourly_summary",
      "dashboard_var": "__SUMMARY_TABLE_HOURLY__",
      "parent_table_name": "cloudfront_kinesis",
      "requires_functions": ["lookup_city"],
      "sql": {
        "path": "sql/hourly_summary.sql",
        "sha256": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890"
      }
    },
    {
      "name": "cloudfront_daily_summary",
      "dashboard_var": "__SUMMARY_TABLE_DAILY__",
      "parent_table_name": "cloudfront_kinesis",
      "sql": {
        "path": "sql/daily_summary.sql",
        "sha256": "b2c3d4e5f67890123456789012345678901234567890123456789012345678901"
      }
    }
  ],
  "ui": {
    "primary_url": "https://cascade-marketplace.s3.us-east-1.amazonaws.com/public/assets/docs/Cloudfront.html",
    "method": {
      "full_title": "Amazon Kinesis",
      "icon_url": "https://cascade-marketplace.s3.us-east-1.amazonaws.com/public/assets/icons/kinesis.png"
    },
    "source": {
      "full_title": "CloudFront",
      "icon_url": "https://cascade-marketplace.s3.us-east-1.amazonaws.com/public/assets/icons/cloudfront.png"
    },
    "data_category": "cdn"
  },
  "metadata": {
    "version": "1.0.0",
    "maintainer": "jgoodson@hydrolix.io",
    "description": "Amazon Kinesis Cloudfront",
    "channel_type": "AWS"
  },
  "dependencies": {
    "grafana": {
      "version": "^12.1.0",
      "plugins": [
        {
          "name": "hydrolix-datasource",
          "version": "^1.0.0",
          "type": "datasource"
        },
        {
          "name": "marcusolsson-treemap-panel",
          "version": "^1.3.0",
          "type": "panel"
        }
      ]
    },
    "hydrolix": {
      "cluster_version": "^5.4.0",
      "required_dictionaries": [
        {
          "name": "geoip_asn",
          "source": "https://geolite.maxmind.com/download/geoip/database/GeoLite2-ASN.tar.gz"
        },
        {
          "name": "geoip_city",
          "source": "https://geolite.maxmind.com/download/geoip/database/GeoLite2-City.tar.gz"
        }
      ],
      "required_functions": [
        {
          "name": "lookup_city",
          "definition": "CREATE FUNCTION lookup_city(ip String) RETURNS String AS dictGet('geoip_city', 'city_name', tuple(IPv4StringToNum(ip)))",
          "requires_dictionaries": ["geoip_city"]
        },
        {
          "name": "lookup_asn",
          "definition": "CREATE FUNCTION lookup_asn(ip String) RETURNS UInt32 AS dictGet('geoip_asn', 'asn', tuple(IPv4StringToNum(ip)))",
          "requires_dictionaries": ["geoip_asn"]
        }
      ]
    },
    "data-sources": []
  }
}
```

### Execution Order for Example Bundle

Based on the dependency graph:

```
Level 0 (parallel):
  - Dictionary: geoip_asn
  - Dictionary: geoip_city

Level 1 (parallel, after Level 0):
  - Function: lookup_city (requires geoip_city)
  - Function: lookup_asn (requires geoip_asn)

Level 2 (after Level 1):
  - Table: cloudfront_kinesis (requires lookup_city, lookup_asn)

Level 3 (parallel, after Level 2):
  - Summary Table: cloudfront_hourly_summary (requires cloudfront_kinesis, lookup_city)
  - Summary Table: cloudfront_daily_summary (requires cloudfront_kinesis)

Grafana Setup (can happen anytime before dashboard deployment):
  - Install plugin: hydrolix-datasource
  - Install plugin: marcusolsson-treemap-panel

Final:
  - Deploy dashboard (requires plugins installed)
```

## Changes from v1

**New Fields**:
- `Function.requires_dictionaries` - Explicit dictionary dependencies for functions
- `Table.requires_functions` - Explicit function dependencies for tables
- `Table.requires_dictionaries` - Explicit dictionary dependencies for tables
- `SummaryTable.requires_functions` - Explicit function dependencies for summary tables
- `SummaryTable.requires_dictionaries` - Explicit dictionary dependencies for summary tables
- `Dashboard.requires_plugins` - Explicit Grafana plugin dependencies for dashboards
- `GrafanaPlugin.type` - Optional plugin type classification

**New Validation Rules**:
- Dependency reference integrity validation
- Acyclic dependency graph validation
- Execution order computation via topological sort

**Purpose**:
These changes enable automated dependency resolution and ensure resources are created in the correct order, preventing runtime errors due to missing dependencies.
