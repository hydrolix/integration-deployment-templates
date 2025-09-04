# Bundle Testing How-To Guide

This guide explains how to test bundles using the Rust-based testing tool.

## Prerequisites

### Required Software
- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Docker**: Required for local Grafana testing
- **Chrome/Chromium**: Required for headless browser testing

### Required Environment Variables
Set these environment variables before running tests:

```bash
export BUNDLE_TESTING_CLUSTER="your-cluster-endpoint"
export BUNDLE_TESTING_USERNAME="your-username"
export BUNDLE_TESTING_PASSWORD="your-password"
```

## Basic Usage

### 1. Run All Bundle Tests (Validation Only)
```bash
cargo run
```
This runs validation checks on all `bundle.json` files found in the current directory and subdirectories.

### 2. Test a Specific Bundle
```bash
cargo run -- bundle_name_filter
```
Only tests bundles whose names contain the specified filter string.

**Example:**
```bash
cargo run -- cloudfront
```
This will only test bundles with "cloudfront" in their name.

### 3. Full Local Testing with Grafana
```bash
cargo run -- --local
```
This performs complete end-to-end testing including:
- All validation checks
- Grafana container deployment
- Dashboard rendering verification
- Browser-based error detection

### 4. Generate Output for Traffic Generation
```bash
cargo run -- --output
```
Dumps detailed output in JSON format for use in traffic generation.

### 5. Marketplace Testing
```bash
cargo run -- --marketplace
```
Runs tests specifically for marketplace bundles.

## Command Line Options

| Flag | Description |
|------|-------------|
| `--local` | Enable full integration testing with local Grafana container |
| `--marketplace` | Enable marketplace-specific testing |
| `--output` | Dump detailed output in JSON format |
| `[filter]` | Test only bundles containing this string in their name |

## What Gets Tested

### Validation-Only Tests (Always Run)

#### Structural Validation
- ✅ URL format validation (must start with `https://` or `file://`)
- ✅ Path format validation (no leading `/`, no `..`, proper extensions)
- ✅ Macro variable format (`__VARIABLE_NAME__`)
- ✅ Enum validation (method, data_category, channel_type)
- ✅ SHA256 checksum format validation
- ✅ Bundle name format (alphanumeric, underscores, dashes only)

#### Business Logic Validation
- ✅ No duplicate table names or dashboard variables
- ✅ Naming consistency (method titles, WAF sources, etc.)
- ✅ File integrity (checksums, file existence)
- ✅ Dashboard JSON structure and required variables
- ✅ Transform content validation
- ✅ Sample data existence

### Local Integration Tests (Only with `--local`)

#### Grafana Container Testing
- ✅ Automatic container cleanup and restart
- ✅ Bundle deployment to Grafana
- ✅ 30-second startup wait period

#### Browser Testing
- ✅ Headless Chrome dashboard rendering
- ✅ Datasource error detection
- ✅ "No data" error detection
- ✅ Zero-error requirement for success

## Test Workflow

### Basic Validation Workflow
1. **Discovery**: Finds all `bundle.json` files (max depth 2)
2. **Filtering**: Applies name filter if specified
3. **Parsing**: Loads and validates JSON structure
4. **Validation**: Runs all validation checks
5. **Results**: Reports success or failure

### Local Testing Workflow
1. **Basic validation** (steps 1-4 above)
2. **Container setup**: Kills existing Grafana, starts fresh
3. **Deployment**: Deploys bundle to Grafana
4. **Browser testing**: Loads dashboard in headless Chrome
5. **Error scanning**: Checks for datasource/data errors
6. **Cleanup**: Container management

## Common Test Scenarios

### Testing During Development
```bash
# Quick validation check
cargo run -- my_bundle

# Full local testing
cargo run -- --local my_bundle
```

### Testing All Bundles Before Release
```bash
# Validate all bundles
cargo run

# Full integration test for all bundles
cargo run -- --local
```

### Debugging Failed Tests
```bash
# Get detailed output
cargo run -- --output my_bundle

# Test specific bundle with full logging
cargo run -- --local my_bundle
```

## Understanding Test Results

### Success Output
```
Testing my_bundle_name
Bundle=Bundle { name: "my_bundle_name", ... }
SUCCESS
Success
```

### Failure Output
```
ERROR: Failed bundle validation: Found duplicate tokens: error=...
```

### Browser Test Results
```
Dashboard Errors=0 NoDataErrors=0  // ✅ Success
Dashboard Errors=2 NoDataErrors=1  // ❌ Failure
```

## File Structure Requirements

Your bundle directory should look like:
```
my_bundle/
├── bundle.json          # Required: Bundle configuration
├── dashboard.json       # Required: Grafana dashboard
├── transforms/          # Optional: Transform files
│   └── transform1.json
└── sample_data/         # Required: Sample data files
    └── sample.json
```

## Troubleshooting

### Environment Variable Issues
```bash
# Check if variables are set
echo $BUNDLE_TESTING_CLUSTER
echo $BUNDLE_TESTING_USERNAME
echo $BUNDLE_TESTING_PASSWORD
```

### Docker Issues
```bash
# Check Docker is running
docker ps

# Manual container cleanup
docker stop grafana-container-name
docker rm grafana-container-name
```

### Bundle Not Found
- Ensure `bundle.json` exists in current directory or subdirectories
- Check that the bundle name filter matches exactly
- Verify file permissions

### Browser Testing Failures
- Ensure Chrome/Chromium is installed and accessible
- Check that Grafana container started successfully
- Verify dashboard deploys without errors

## Best Practices

1. **Always run validation first**: Use `cargo run` before `--local` testing
2. **Test specific bundles**: Use name filters during development
3. **Clean environment**: Ensure no conflicting Docker containers
4. **Check logs**: Review all error messages for specific failure details
5. **Incremental testing**: Fix validation errors before running integration tests

## Exit Codes

- **0**: All tests passed successfully
- **1**: Test failure (validation error, deployment failure, or dashboard errors)

The tool exits immediately on the first failure encountered.