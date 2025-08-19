# Bundle Format Documentation

A bundle is a Hydrolix JSON configuration file that packages transformations, dashboards, and documentation for data integration and visualization. 

This document describes all valid fields and their validation rules.

## Root Bundle Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | ✅ | Bundle identifier. Must contain both `source` and `method` values |
| `source` | `string` | ✅ | Data source type. See [Valid Sources](#valid-sources) |
| `method` | `string` | ✅ | Integration method. See [Valid Methods](#valid-methods) |
| `beta` | `boolean` | ✅ | Whether this is a beta release |
| `base_url` | `string` | ✅ | HTTPS URL to the repository base path |
| `dashboard` | `Dashboard` | ✅ | Dashboard configuration |
| `tables` | `Table[]` | ✅ | Array of table definitions |
| `ui` | `Ui` | ✅ | User interface configuration |
| `metadata` | `Metadata` | ✅ | Bundle metadata |
| `method_overrides` | `MethodOverrides` | - | Optional method-specific overrides |

### Validation Rules for Root Object
- `base_url` must start with `https://`
- `name` must contain both the `source` and `method` values as substrings
- No duplicate table names across all tables
- No duplicate `dashboard_var` values across all tables

## Dashboard Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` | ✅ | Relative path to dashboard JSON file |
| `project_var` | `string` | ✅ | Variable placeholder for project name |

### Validation Rules for Dashboard
- `path` cannot start with `/`
- `path` cannot contain `..`
- `path` must end with `.json` or `.tsv`
- `project_var` must follow macro format: `__VARIABLE_NAME__`

## Table Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dashboard_var` | `string` | ✅ | Variable placeholder for table name in dashboard |
| `name` | `string` | ✅ | Table identifier |
| `transforms` | `Transform[]` | ✅ | Array of transformation definitions |

### Validation Rules for Table
- `dashboard_var` must follow macro format: `__VARIABLE_NAME__`
- `name` must be unique across all tables in the bundle

## Transform Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `data_template_path` | `string` | ✅ | Relative path to sample data template |
| `path` | `string` | ✅ | Relative path to transformation JSON |

### Validation Rules for Transform
- Both paths cannot start with `/`
- Both paths cannot contain `..`
- Both paths must end with `.json` or `.tsv`

## Ui Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `primary_url` | `string` | ✅ | HTTPS URL to primary documentation |
| `method` | `Graphics` | ✅ | Method display information |
| `source` | `Graphics` | ✅ | Source display information |
| `data_category` | `string` | ✅ | Data category classification |

### Validation Rules for UI
- `primary_url` must start with `https://`
- `data_category` must be one of: `"video"`, `"cdn"`, `"security"`

## Graphics Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `full_title` | `string` | ✅ | Display title for the component |
| `icon_url` | `string` | ✅ | HTTPS URL to icon image |

### Validation Rules for Graphics
- `icon_url` must start with `https://`

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
| `region` | `string` | ✅ | AWS region override |
| `stream_prefix` | `string` | ✅ | Stream name prefix |

### Validation Rules for MethodOverrides
- `region` must be exactly `"us-east-1"`

## Valid Sources

The `source` field must be one of:
- `"waf"` - Web Application Firewall
- `"cloudtrail"` - AWS CloudTrail
- `"vpc"` - Virtual Private Cloud
- `"alb"` - Application Load Balancer
- `"cloudfront"` - Amazon CloudFront
- `"medialive"` - AWS Elemental MediaLive
- `"mediatailor"` - AWS Elemental MediaTailor
- `"mediapackage"` - AWS Elemental MediaPackage

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
- Inner content must be uppercase letters and single underscores only
- No consecutive underscores within the variable name
- Minimum 5 characters total (e.g., `__X__`)

**Examples**:
- ✅ `__PROJECT_NAME__`
- ✅ `__TABLE_NAME__`
- ✅ `__DATA_SOURCE__`
- ❌ `_PROJECT_` (single underscores)
- ❌ `__project_name__` (lowercase)
- ❌ `__PROJECT__NAME__` (consecutive underscores)

## URL Validation

HTTPS URLs must:
- Start with `https://`
- Be valid URLs according to URL parsing standards
- Be accessible (return successful HTTP response)

Path fields must:
- Not start with `/`
- Not contain `..` (directory traversal)
- End with `.json` or `.tsv`

## Consistency Validation

Additional validation rules ensure consistency across the bundle:

1. **Method Title Consistency**: The `ui.method.full_title` should contain expected titles based on the method:
   - `firehose`: "Amazon Data Firehose", "AWS Firehose", or "Kinesis Data Firehose"
   - `s3`: "Amazon S3" or "AWS S3"
   - `kinesis`: "Amazon Kinesis" or "AWS Kinesis"

2. **WAF Source Rule**: If `source` is `"waf"`, the `ui.source.full_title` must contain "WAF" (case-insensitive)

3. **Transformation Validation**: All transformation files referenced in `transforms[].path` must:
   - Be valid JSON
   - Contain a non-empty `name` field
   - If they have a subtype, it must be `"firehose"`

## Example Bundle

```json
{
  "name": "kinesis-cloudfront",
  "source": "cloudfront",
  "method": "kinesis",
  "beta": true,
  "base_url": "https://github.com/example/repo/blob/main/templates",
  "dashboard": {
    "path": "dashboards/current.json",
    "project_var": "__PROJECT_NAME__"
  },
  "tables": [
    {
      "dashboard_var": "__TABLE_NAME__",
      "name": "cloudfront_kinesis",
      "transforms": [
        {
          "data_template_path": "transformations/sample_data.tsv",
          "path": "transformations/current.json"
        }
      ]
    }
  ],
  "ui": {
    "primary_url": "https://docs.example.com/cloudfront.html",
    "method": {
      "full_title": "Amazon Kinesis",
      "icon_url": "https://cdn.example.com/kinesis.png"
    },
    "source": {
      "full_title": "CloudFront",
      "icon_url": "https://cdn.example.com/cloudfront.png"
    },
    "data_category": "cdn"
  },
  "metadata": {
    "version": "1.0.0",
    "maintainer": "user@example.com",
    "description": "Amazon Kinesis CloudFront integration",
    "channel_type": "AWS"
  }
}
```
