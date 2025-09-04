# Bundle Testing Checks

This document describes all validation checks performed when running bundle tests locally (`cargo run -- --local`).

## Structural Validation (During JSON Parsing)

These checks happen automatically when the `bundle.json` file is parsed:

### URL Format Validation
- ✅ `base_url` must start with `https://` or `file://`
- ✅ `ui.primary_url` must start with `https://` or `file://`
- ✅ `ui.method.icon_url` must start with `https://` or `file://`
- ✅ `ui.source.icon_url` must start with `https://` or `file://`
- ✅ Dictionary source URLs must start with `https://` or `file://`

### Path Format Validation
- ✅ Dashboard and transform paths cannot start with `/`
- ✅ Paths cannot contain `..` (directory traversal protection)
- ✅ Dashboard path must end with `.json` or `.tsv`
- ✅ Transform paths must end with `.json` or `.tsv`

### Macro Variable Format
- ✅ `dashboard.project_var` must follow `__VARIABLE_NAME__` format
- ✅ `tables[].dashboard_var` must follow `__VARIABLE_NAME__` format
- ✅ Macro variables must be uppercase with proper underscore format

### Enum Validation
- ✅ `method` must be one of: `firehose`, `s3`, `kinesis`, `lambda`, `http_streaming`, `http`
- ✅ `ui.data_category` must be one of: `video`, `cdn`, `security`
- ✅ `metadata.channel_type` must be one of: `AWS`, `Azure`, `GCP`, `3rdParty`, `Internal`
- ✅ `method_overrides.region` must be exactly `us-east-1` (if present)

### Format Validation
- ✅ SHA256 values must be exactly 64 hexadecimal characters
- ✅ Bundle name can only contain alphanumeric characters, underscores, and dashes
- ✅ Source can contain any characters except whitespace and punctuation

## Business Logic Validation

### Duplicate Detection
- ✅ **No duplicate table names** across all tables in the bundle
- ✅ **No duplicate dashboard_var values** across all tables

### Naming Consistency
- ✅ **Method title consistency**: UI method title must match the method type
  - `firehose` → must contain "Amazon Data Firehose", "AWS Firehose", or "Kinesis Data Firehose"
  - `s3` → must contain "Amazon S3" or "AWS S3"  
  - `kinesis` → must contain "Amazon Kinesis" or "AWS Kinesis"
- ✅ **WAF source rule**: If source is "waf", the source title must contain "WAF"
- ✅ **Name contains source and method**: Bundle name must include both source and method values
- ✅ **Version format**: Must follow semantic versioning (e.g., "1.0.0")
- ✅ **Email format**: Maintainer must be a valid email address
- ✅ **Description**: Cannot be empty or whitespace only

### File Integrity
- ✅ **Checksum validation**: SHA256 checksums are verified for dashboard and transform files (if provided)
- ✅ **File existence**: All referenced dashboard and transform files must exist

### Dashboard Validation
- ✅ **Valid JSON structure**: Dashboard file must be valid JSON
- ✅ **Proper dashboard structure**: Must have top-level "dashboard" object
- ✅ **No ID conflicts**: Dashboard cannot have "id" field pre-set
- ✅ **Required variables**: Dashboard must contain:
  - `__DASHBOARD_UUID__`
  - `__DATASOURCE__`
  - `__PROJECT_NAME__`
  - All table `dashboard_var` values from the bundle

### Transform Content Validation
- ✅ **Valid JSON**: All transform files must be valid JSON
- ✅ **Required name field**: Each transform must have a non-empty "name" field
- ✅ **Subtype validation**: If "subtype" field exists, it must be "firehose"

### Sample Data
- ✅ **Sample data exists**: Required sample data files must be present

## Local Integration Testing

When running with `--local` flag, additional end-to-end testing is performed:

### Grafana Container Testing
- ✅ **Container management**: Kills any existing Grafana container and starts fresh
- ✅ **Deployment testing**: Deploys the bundle to local Grafana instance
- ✅ **Startup validation**: Waits for Grafana to be ready (30-second startup delay)

### Headless Browser Testing
- ✅ **Dashboard rendering**: Uses headless Chrome to load the deployed dashboard
- ✅ **Error detection**: Scans for:
  - Datasource connection errors
  - "No data" errors in panels
- ✅ **Zero-error requirement**: Any dashboard errors cause the test to fail

### Container Cleanup
- ✅ **Cleanup**: Stops and removes Grafana container after testing

## Exit Conditions

**Success**: Test passes only if ALL checks pass and dashboard renders without errors

**Failure**: Test fails immediately on:
- Any structural validation error during JSON parsing
- Any business logic validation failure  
- File integrity issues (missing files, bad checksums)
- Dashboard deployment failures
- Any dashboard errors detected in browser testing

## Environment Requirements

For local testing you need:
- Docker (for Grafana container)
- Chrome/Chromium (for headless browser testing)
- Required environment variables:
  - `BUNDLE_TESTING_CLUSTER`
  - `BUNDLE_TESTING_USERNAME` 
  - `BUNDLE_TESTING_PASSWORD`