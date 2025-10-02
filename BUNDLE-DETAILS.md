# Bundle Format Documentation

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

## Dashboard Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to dashboard JSON or TSV file |
| `project_var` | `string` | ✅ | Variable placeholder for project name |
| `sha256` | `string` | ❌ | Optional SHA256 hash of dashboard contents (64 hex characters) |

### Validation Rules for Dashboard
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json` or `.tsv`
- `project_var` must follow macro format: `__VARIABLE_NAME__`
- Use `openssl dgst -sha256 <file_name>` to generate the sha256

## Table Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dashboard_var` | `string` | ✅ | Variable placeholder for table name in dashboard |
| `name` | `string` | ✅ | Table identifier |
| `transforms` | `Transform[]` | ✅ | Array of transformation definitions |

### Validation Rules for Table
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique across all tables in the bundle

## SummaryTable Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Summary table identifier |
| `dashboard_var` | `string` | ✅ | Variable placeholder for summary table name in dashboard |
| `source_table` | `string` | ✅ | Variable placeholder for source table reference |
| `sql` | `SummarySqlFile` | ✅ | SQL file configuration for summary table |

### Validation Rules for SummaryTable
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `source_table` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique across all summary tables in the bundle

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
| `name` | `string` | ✅  | Plugin identifier |
| `version` | `string` | ✅  | Plugin version requirement |

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

## Example Bundle with Summary Tables and Dependencies

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
    "sha256": "65d22b569bb986a28e98246637bd41dad5ecf56220965d2cc3491577a160138b"
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
      "dashboard_var": "__SUMMARY_TABLE__",
      "source_table": "__TABLE_NAME__",
      "sql": {
        "path": "sql/hourly_summary.sql",
        "sha256": "a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890"
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
          "name": "grafana-clickhouse-datasource",
          "version": "^4.10.1"
        }
      ]
    },
    "hydrolix": {
      "cluster_version": "^5.4.0",
      "required_dictionaries": [
        {
          "name": "geoip_asm",
          "source": "https://geolite.maxmind.com/download/geoip/database/GeoLite2-ASN.tar.gz"
        },
        {
          "name": "geoip_city",
          "source": "https://geolite.maxmind.com/download/geoip/database/GeoLite2-City.tar.gz"
        }
      ],
      "required_functions": []
    },
    "data-sources": []
  }
}
```