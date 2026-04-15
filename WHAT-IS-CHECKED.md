# Test Coverage - Bundle Validation System

This document outlines the comprehensive validation checks performed by the bundle validation system. The system ensures that integration deployment bundles meet all quality, consistency, and functional requirements before deployment.

## Overview

The bundle checker performs validation in two phases:
1. **Individual Bundle Validation** - Each bundle is validated independently
2. **Global Cross-Bundle Validation** - All bundles are validated together for conflicts

## Individual Bundle Validation

### 1. Base URL Validation (`valid_base_url.rs`)

**Purpose**: Ensures bundle references the correct GitHub repository location.

**Checks**:
- Validates that `bundle.base_url` matches the expected GitHub URL format
- Expected format: `https://github.com/hydrolix/integration-deployment-templates/blob/main/{base}`
- Prevents bundles from referencing incorrect or malicious repositories

**Failure Conditions**:
- Base URL doesn't match the expected GitHub repository pattern

### 2. Transform File Validation (`transforms_are_valid.rs`)

**Purpose**: Validates all transformation files referenced in the bundle.

**Checks**:
- **File Accessibility**: All transform files can be read from disk
- **JSON Validity**: Transform files contain valid JSON syntax
- **Required Fields**:
  - `name` field exists and is a non-empty string
  - `subtype` field (if present) must equal "firehose"
- **Data Integrity**: Transform file structure matches expected schema

**Failure Conditions**:
- Transform file cannot be read
- Invalid JSON syntax in transform file
- Missing or empty `name` field
- Invalid `subtype` value (must be "firehose" if present)
- `name` field is not a string type

### 3. Sample Data Validation (`sample_data_exists.rs`)

**Purpose**: Ensures all transforms include sample data for testing and validation.

**Checks**:
- **Sample Data Presence**: Each transform contains `settings.sample_data`
- **Data Format**: Sample data is either:
  - A non-empty JSON object
  - A non-empty string (after trimming whitespace)
- **Content Validation**: Sample data contains meaningful test data

**Failure Conditions**:
- Missing `sample_data` field in transform settings
- Empty sample data object or string
- Sample data field exists but contains only whitespace

### 4. Sample Data Freshness (`sample_data_freshness.rs`)

**Purpose**: Warns when sample data contains stale timestamps that may cause Grafana dashboard panels to appear empty due to default time range filters.

**Checks**:
- **Primary Timestamp Detection**: Identifies the primary epoch column from the transform's `output_columns` schema (where `datatype.type == "epoch"` and `datatype.primary == true`)
- **Staleness Check**: Compares the primary timestamp value in `sample_data` against the current time
- **Threshold**: Timestamps older than 6 months (183 days) are flagged as stale

**Behavior**:
- **Warning only** — does not fail validation. Stale timestamps are logged as warnings but do not block merges.
- On the full pipeline track (Track 1), the Python configurator automatically shifts stale timestamps to the 1st of the current month before this validator runs.
- On the validation-only track (Track 2), this check serves as a safety net to surface staleness.

**Skipped when**:
- No primary epoch column exists in the transform schema
- The primary timestamp value is not numeric in sample_data
- The `output_columns` field is missing from the transform

### 5. Duplicate Token Validation (`no_duplicate_tokens.rs`)

**Purpose**: Prevents naming conflicts within a single bundle.

**Checks**:
- **Table Name Validation**:
  - No duplicate table names within the bundle
  - Table names start with a letter (alphabetic character)
  - Table names contain only alphanumeric characters and underscores
- **Dashboard Variable Validation**:
  - No duplicate `dashboard_var` values within the bundle
  - Each table has a unique dashboard variable identifier

**Failure Conditions**:
- Duplicate table names within the same bundle
- Table name doesn't start with a letter
- Table name contains invalid characters (not alphanumeric or underscore)
- Duplicate dashboard variable values

### 6. Checksum Validation (`no_bad_checksums.rs`)

**Purpose**: Ensures file integrity through SHA256 checksum verification.

**Checks**:
- **Dashboard Files**: Validates SHA256 checksums for dashboard files
- **Transform Files**: Validates SHA256 checksums for all transform files
- **File Integrity**: Computed checksums match declared checksums
- **File Accessibility**: All files can be read for checksum computation

**Failure Conditions**:
- File cannot be read for checksum calculation
- Computed SHA256 doesn't match declared checksum
- Missing checksum when expected

### 7. Naming Convention Validation (`naming_is_valid.rs`)

**Purpose**: Enforces consistent naming conventions across bundle components.

**Checks**:
- **Method-Title Consistency**:
  - `firehose` method: UI title contains "Amazon Data Firehose", "AWS Firehose", or "Kinesis Data Firehose"
  - `s3` method: UI title contains "Amazon S3" or "AWS S3"
  - `kinesis` method: UI title contains "Amazon Kinesis" or "AWS Kinesis"
- **Source-Title Consistency**:
  - WAF source: UI title must contain "WAF" (case-insensitive)
- **Bundle Name Requirements**:
  - Bundle name includes both source and method (case-insensitive)
- **Version Format**: Semantic versioning (X.Y.Z format)
- **Maintainer Format**: Valid email address format
- **Description**: Non-empty description field

**Failure Conditions**:
- Method and UI title mismatch
- WAF source without "WAF" in title
- Bundle name missing source or method
- Invalid semantic version format
- Invalid email format for maintainer
- Empty description

### 8. Dashboard Validation (`dashboard_is_valid.rs`)

**Purpose**: Validates Grafana dashboard files and their template variables.

**Checks**:
- **File Accessibility**: Dashboard file can be read
- **JSON Validity**: Dashboard contains valid JSON
- **Required Placeholders**:
  - `__DASHBOARD_UUID__` - Dashboard identifier placeholder
  - `__DATASOURCE__` - Data source placeholder
  - `__PROJECT_NAME__` - Project name placeholder
  - All table dashboard variables from bundle configuration
- **Dashboard Structure**:
  - Top-level `dashboard` object exists
  - No hardcoded `id` field (must use placeholder)

**Failure Conditions**:
- Dashboard file cannot be read
- Invalid JSON in dashboard file
- Missing required placeholder variables
- Dashboard lacks required structure
- Hardcoded ID present instead of placeholder

## Global Cross-Bundle Validation

### 9. Global Duplicate Prevention (`no_global_duplicates.rs`)

**Purpose**: Prevents conflicts across all bundles in the repository.

**Checks**:
- **Bundle Name Uniqueness**: No two bundles share the same name
- **UI Source Title Uniqueness**: No duplicate source titles in UI configuration
- **Table Name Uniqueness**: No table names are duplicated across all bundles

**Failure Conditions**:
- Multiple bundles with identical names
- Multiple bundles with identical UI source titles
- Multiple bundles declaring tables with identical names

## Integration Testing (Local Mode)

When run with `--local` flag, the system performs additional functional testing:

### Dashboard Deployment Testing
- Deploys dashboard to local Grafana instance
- Validates dashboard renders without errors
- Checks for missing datasource configurations

### Headless Browser Testing (`headless_browser.rs`)
- Automated Grafana dashboard testing using headless Chrome
- **Error Detection**:
  - Datasource errors (regex: `Datasource \w+ was not found`)
  - No-data errors (regex: `api/ds/query\?ds_type=[^&]+-clickhouse-datasource`)
- **Visual Validation**: 30-second load time to ensure all panels render
- **Authentication**: Automatic Grafana session management

### Data Integration Testing (`hdx.rs`)
- Creates test tables in Hydrolix platform
- Deploys transforms to test environment
- Ingests sample data using configured transforms
- Validates end-to-end data flow

## Test Execution Flow

1. **Discovery**: Finds all `bundle.json` files in repository
2. **Individual Validation**: Each bundle undergoes all validation checks
3. **Global Validation**: Cross-bundle conflict detection
4. **Integration Testing** (if enabled): Functional testing with live systems
5. **Reporting**: Detailed error reporting with file locations and line numbers

## Error Handling

All validation functions provide detailed error messages including:
- Source file and line number for debugging
- Specific failure reason
- Relevant file paths and configuration values
- Actionable guidance for resolution

The system exits with non-zero status code on any validation failure, making it suitable for CI/CD pipeline integration.
