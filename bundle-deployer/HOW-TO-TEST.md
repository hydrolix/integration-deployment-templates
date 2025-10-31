# Bundle Testing and Deployment Guide

This guide explains how to validate, test, and deploy bundles using the TypeScript/Deno Bundle Deployer.

## Prerequisites

### Required Software
- **Deno**: Install from [deno.land](https://deno.land/)
- **Docker**: Required for local Grafana testing
- **Chrome/Chromium**: Required for headless browser testing
- **unzip**: For automatic dictionary zip extraction (usually pre-installed)

### Required Environment Variables

Set these environment variables before running any commands:

```bash
export BUNDLE_TESTING_CLUSTER="partnersandbox.trafficpeak.live"
export BUNDLE_TESTING_USERNAME="your-username"
export BUNDLE_TESTING_PASSWORD="your-password"
```

**Note**: These credentials are used to:
- Authenticate with Hydrolix cluster
- Create test resources (tables, functions, dictionaries)
- Deploy and validate bundles

## Basic Usage

### 1. Validate Bundle (Fast - No Deployment)

```bash
deno run --allow-all src/main.ts [bundle_name]
```

This performs fast validation checks without deploying anything:
- ✅ Bundle structure validation
- ✅ File existence checks
- ✅ JSON syntax validation
- ✅ Template variable verification
- ✅ SQL reference validation
- ✅ Function/dictionary file checks
- ✅ No deployment or data creation

**Example:**
```bash
deno run --allow-all src/main.ts mcdn_test
```

**When to use**: During development, before committing changes, in CI/CD pipelines.

**Time**: ~30 seconds

---

### 2. Full Local Deployment with Testing

```bash
deno run --allow-all src/main.ts --local [bundle_name]
```

This performs complete end-to-end testing:
- ✅ All validation checks (same as above)
- ✅ Extracts dictionary zips automatically
- ✅ Creates functions and dictionaries in Hydrolix
- ✅ Creates tables with transforms
- ✅ Inserts sample data
- ✅ Creates summary tables (if defined)
- ✅ Deploys Grafana datasource
- ✅ Deploys all dashboards
- ✅ Creates alert rules (if defined)
- ✅ Headless Chrome testing (30-second dashboard load)
- ✅ Error detection (datasource errors, no-data errors)

**Example:**
```bash
deno run --allow-all src/main.ts --local mcdn_test
```

**When to use**: Final testing before release, validating end-to-end functionality.

**Time**: ~3-5 minutes (depends on data volume and cluster response time)

---

### 3. Dashboard-Only Deployment

```bash
deno run --allow-all src/main.ts --local-dashboard-only [bundle_name]
```

This deploys dashboards without creating tables or data:
- ✅ All validation checks
- ✅ Deploys Grafana datasource
- ✅ Deploys all dashboards
- ✅ Creates alert rules (if defined)
- ✅ Headless Chrome testing
- ❌ **Does NOT** create tables, functions, dictionaries, or insert data

**Example:**
```bash
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test
```

**When to use**: Testing dashboard changes without recreating tables, iterating on visualizations.

**Time**: ~1-2 minutes

---

### 4. Production Mode Validation

```bash
deno run --allow-all src/main.ts --production [bundle_name]
```

This validates dependencies exist on the target cluster:
- ✅ All validation checks
- ✅ Checks functions exist on cluster (with correct project prefix)
- ✅ Checks dictionaries exist on cluster (with correct project prefix)
- ✅ Verifies local definition files present
- ❌ **Does NOT** deploy anything

**Example:**
```bash
deno run --allow-all src/main.ts --production mcdn_test
```

**When to use**: Validating bundle readiness for production deployment, CI/CD pre-deployment checks.

**Time**: ~30-60 seconds

---

### 5. Filter by Bundle Name

```bash
deno run --allow-all src/main.ts [bundle_name]
```

Test only bundles whose names contain the specified string:

```bash
# Only test mcdn bundles
deno run --allow-all src/main.ts mcdn

# Only test cloudfront bundles
deno run --allow-all src/main.ts cloudfront

# Test all bundles (no filter)
deno run --allow-all src/main.ts
```

---

### 6. Generate Deployment Output

```bash
deno run --allow-all src/main.ts --output [bundle_name]
```

Dumps detailed deployment information in JSON format:
- Cluster domain
- Project name
- Grafana domain and datasource UID
- Dashboard ID
- Table names and transform details

**Example:**
```bash
deno run --allow-all src/main.ts --output --local mcdn_test
```

**When to use**: Traffic generation, integration with other tools, debugging deployments.

---

### 7. Cleanup Bundle Resources

```bash
# Delete all resources for a bundle (SAFE - bundle-scoped)
deno run --allow-all src/cleanup.ts --all [bundle_name]

# Delete only functions
deno run --allow-all src/cleanup.ts --functions [bundle_name]

# Delete only dictionaries (definitions, not uploaded files)
deno run --allow-all src/cleanup.ts --dictionaries [bundle_name]

# Delete only dictionary files (uploaded CSV/YAML files)
deno run --allow-all src/cleanup.ts --dictionary-files [bundle_name]

# Delete only tables
deno run --allow-all src/cleanup.ts --tables [bundle_name]

# Dry run (see what would be deleted without deleting)
deno run --allow-all src/cleanup.ts --all [bundle_name] --dry-run
```

**Examples:**
```bash
# Safe cleanup - only deletes mcdn_test resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# Preview cleanup
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

# Cleanup specific resource types
deno run --allow-all src/cleanup.ts --functions mcdn_test
deno run --allow-all src/cleanup.ts --dictionaries mcdn_test
```

**When to use**: After testing, before redeploying with changes, cleaning up failed deployments.

**⚠️ Warning**: Running cleanup without specifying a bundle name will delete **ALL** resources in the project. Always specify the bundle name!

```bash
# DANGEROUS - Deletes everything!
deno run --allow-all src/cleanup.ts --all
```

---

## Command Line Options

| Flag | Description | Use Case |
|------|-------------|----------|
| `--local` | Full deployment with testing | Final validation, production readiness |
| `--local-dashboard-only` | Dashboard deployment only | Dashboard iteration, visualization testing |
| `--production` | Validate dependencies exist | Pre-deployment validation, CI/CD |
| `--output` | Dump deployment info as JSON | Traffic generation, integration, debugging |
| `[bundle_name]` | Filter by bundle name | Test specific bundles during development |

---

## What Gets Validated and Tested

### Validation Phase (Always Run)

#### Structural Validation
✅ Bundle JSON structure and required fields  
✅ URL format validation (`https://` or `file://`)  
✅ Path validation (no `/`, no `..`, proper extensions)  
✅ Macro variable format (`__VARIABLE_NAME__`)  
✅ Enum validation (method, source, channel_type, data_category)  
✅ SHA256 checksum format (64 hex characters)  
✅ Bundle name format (alphanumeric, underscores, dashes)

#### Content Validation
✅ No duplicate table names or dashboard variables  
✅ Naming consistency (method titles, source names)  
✅ File existence and accessibility  
✅ Transform JSON validity and structure  
✅ Sample data presence in transforms  
✅ Dashboard JSON structure and template variables  
✅ Alert rules JSON structure (if present)  
✅ Summary table SQL files (if present)

#### Dependency Validation
✅ Function files exist for declared functions  
✅ Dictionary files exist (both `.json` + data file)  
✅ SQL references match declared dependencies  
✅ Template variables used correctly (`__PROJECT_NAME__`)

---

### Deployment Phase (With `--local` or `--local-dashboard-only`)

#### Resource Creation (`--local` only)
✅ Extract `dictionaries.zip` automatically  
✅ Auto-discover functions from `functions/` directory  
✅ Auto-discover dictionaries from extracted files  
✅ Create functions in Hydrolix (with project prefix)  
✅ Upload dictionary files (CSV, YAML, etc.)  
✅ Create dictionary definitions in Hydrolix  
✅ Create tables in Hydrolix  
✅ Add transforms to tables  
✅ Insert sample data into tables  
✅ Create summary tables (if defined)

#### Grafana Deployment (Both modes)
✅ Kill existing Grafana container (cleanup)  
✅ Start fresh Grafana container  
✅ Wait for Grafana to be ready (health check)  
✅ Create Hydrolix datasource  
✅ Deploy primary dashboard  
✅ Deploy additional dashboards (if defined)  
✅ Create alert rules (if defined)

#### Browser Testing (Both modes)
✅ Load dashboard in headless Chrome  
✅ Wait 30 seconds for all panels to render  
✅ Detect datasource errors (`Datasource \w+ was not found`)  
✅ Check for query errors (400 status codes)  
✅ Verify zero errors for success

---

## Test Execution Flow

### Validation-Only Flow
```
1. Discovery
   └── Find bundle.json in my-bundles/[bundle_name]/

2. Parsing & Structure Validation
   ├── Parse bundle.json
   ├── Validate required fields
   └── Validate field formats

3. File Validation
   ├── Check all referenced files exist
   ├── Validate JSON syntax
   ├── Verify checksums (if provided)
   └── Check sample data presence

4. Content Validation
   ├── Validate dashboard structure
   ├── Check template variables
   ├── Validate transforms
   ├── Check function/dictionary references
   └── Validate alert rules (if present)

5. Cross-Bundle Validation
   ├── Check for duplicate bundle names
   ├── Check for duplicate UI titles
   └── Check for duplicate table names

6. Success/Failure Report
```

---

### Full Deployment Flow (`--local`)
```
1. Validation (same as above)

2. Dependency Extraction
   └── Extract dictionaries.zip to .extracted/ (if exists)

3. Auto-Discovery (if dependencies empty)
   ├── Scan functions/ for .json files
   └── Scan dictionaries/ and .extracted/ for dictionary pairs

4. Hydrolix Resource Creation
   ├── Create functions (with __PROJECT_NAME__ replacement)
   ├── Upload dictionary data files
   ├── Create dictionary definitions
   ├── Create tables
   ├── Add transforms to tables
   ├── Insert sample data (with retry logic)
   └── Create summary tables (if defined)

5. Grafana Setup
   ├── Kill old Grafana container
   ├── Start new Grafana container
   ├── Wait for Grafana ready (60s timeout)
   └── Create Hydrolix datasource

6. Dashboard Deployment
   ├── Process primary dashboard template
   ├── Replace template variables
   ├── Deploy primary dashboard
   ├── Deploy additional dashboards (if any)
   └── Create alert rules (if defined)

7. Browser Testing
   ├── Get Grafana session cookie
   ├── Launch headless Chrome
   ├── Navigate to dashboard
   ├── Wait 30 seconds for rendering
   ├── Monitor console for errors
   └── Report error counts

8. Cleanup
   └── Close browser
```

---

### Dashboard-Only Flow (`--local-dashboard-only`)
```
1. Validation (same as validation-only)

2. Grafana Setup (same as full deployment)

3. Dashboard Deployment (same as full deployment)

4. Browser Testing (same as full deployment)

5. Cleanup (same as full deployment)
```

---

## Common Test Scenarios

### During Development

```bash
# Quick validation check (30 seconds)
deno run --allow-all src/main.ts mcdn_test

# Dashboard changes only (2 minutes)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Full end-to-end test (5 minutes)
deno run --allow-all src/main.ts --local mcdn_test
```

---

### Before Committing

```bash
# Validate structure
deno run --allow-all src/main.ts mcdn_test

# Full test if changing transforms/data
deno run --allow-all src/main.ts --local mcdn_test
```

---

### Testing All Bundles

```bash
# Validate all bundles (fast)
deno run --allow-all src/main.ts

# Full test all bundles (slow)
deno run --allow-all src/main.ts --local
```

---

### Iterating on Dashboards

```bash
# 1. Deploy dashboard
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# 2. Make changes to dashboard JSON

# 3. Clean up and redeploy
deno run --allow-all src/cleanup.ts --all mcdn_test
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test
```

---

### Testing with Fresh Environment

```bash
# 1. Clean up old deployment
deno run --allow-all src/cleanup.ts --all mcdn_test

# 2. Deploy fresh
deno run --allow-all src/main.ts --local mcdn_test
```

---

### Debugging Failed Tests

```bash
# 1. Get detailed output
deno run --allow-all src/main.ts --output --local mcdn_test

# 2. Check specific validation
deno run --allow-all src/main.ts mcdn_test

# 3. Clean up and retry
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
deno run --allow-all src/cleanup.ts --all mcdn_test
deno run --allow-all src/main.ts --local mcdn_test
```

---

## Understanding Test Results

### Validation Success
```
Testing http_streaming_mcdn_test
Base=my-bundles/mcdn_test bundle=Bundle { name: "http_streaming_mcdn_test", ... }
✓ All required dependencies exist on cluster
✓ All required local files present
Final check on all of the bundles for duplicated tokens...
SUCCESS
Success
```

### Validation Failure
```
Testing http_streaming_mcdn_test
ERROR: Failed bundle validation: Transform file is not valid JSON: path=...
```

### Deployment Success (`--local`)
```
Creating table: mcdn_test
Waiting for table to be ready...
✓ Created function city_name
✓ Created dictionary ua_cat_dict
✓ Successfully inserted sample data into sample_project.mcdn_test
✓ Created Grafana datasource with UID: abc123
Starting headless browser test for dashboard: xyz789
Got Grafana session cookie: grafana_session
Page loaded - Title: "CDN Dashboard", URL: http://localhost:3000/d/...
Datasource errors: 0
Success! No datasource errors detected.
Dashboard Errors=0 NoDataErrors=0
SUCCESS
```

### Deployment Failure
```
❌ Transform validation failed (attempt 1/5):
   Status: 400
   Error: Unknown function sample_project_city_name
ERROR: Failed to add transform after 1 attempts
```

### Browser Test Failure
```
ERROR: Datasource not found - Datasource Hydrolix was not found
Datasource errors: 2
Dashboard Errors=2 NoDataErrors=0
ERROR: Dashboard Errors=2 NoDataErrors=0
```

---

## Directory Structure

Your bundle must follow this structure:

```
my-bundles/
└── mcdn_test/
    ├── bundle.json                    # Required: Bundle manifest
    ├── functions/                      # Optional: Custom SQL functions
    │   ├── city_name.json
    │   └── breadcrumbs.json
    ├── dictionaries/                   # Optional: Lookup tables
    │   ├── dictionaries.zip           # Large files (auto-extracted)
    │   ├── .extracted/                # Auto-created (gitignored)
    │   ├── ua_cat_dict.json
    │   └── ua_cat_dict.yaml
    ├── transformations/                # Required: Data schemas
    │   ├── mcdn_akamai_ds2.json
    │   └── mcdn_cloudflare.json
    ├── dashboards/                     # Required: Visualizations
    │   ├── CDN Dashboard.json         # Primary dashboard
    │   ├── alert-rules.json           # Optional: Alert rules
    │   └── Raw Logs.json              # Optional: Additional dashboards
    └── summaries/                      # Optional: Pre-aggregated views
        ├── mcdn_summary_min.sql
        └── mcdn_summary_hour.sql
```

---

## Troubleshooting

### Environment Variable Issues

```bash
# Check if variables are set
echo $BUNDLE_TESTING_CLUSTER
echo $BUNDLE_TESTING_USERNAME
echo $BUNDLE_TESTING_PASSWORD

# Set if missing
export BUNDLE_TESTING_CLUSTER="partnersandbox.trafficpeak.live"
export BUNDLE_TESTING_USERNAME="your-username"
export BUNDLE_TESTING_PASSWORD="your-password"
```

---

### Docker Issues

```bash
# Check Docker is running
docker ps

# Manual Grafana cleanup
docker ps -a | grep grafana
docker stop <container-id>
docker rm <container-id>

# Restart Docker daemon (if needed)
sudo systemctl restart docker  # Linux
# or restart Docker Desktop (macOS/Windows)
```

---

### Bundle Not Found

- Ensure `bundle.json` exists in `my-bundles/[bundle_name]/`
- Check bundle name matches directory name
- Verify file permissions (readable)

```bash
# List available bundles
ls -la my-bundles/
```

---

### Browser Testing Failures

**Datasource errors:**
- Check that datasource was created in Grafana
- Verify Hydrolix cluster is accessible
- Confirm credentials are correct

**Dashboard not loading:**
- Check Grafana container is running: `docker ps`
- Verify Grafana is ready: `curl http://localhost:3000/api/health`
- Review dashboard JSON for template variable issues

**Chrome issues:**
- Ensure Chrome/Chromium is installed
- Try setting `PUPPETEER_EXECUTABLE_PATH` environment variable
- Check Chrome version compatibility

---

### Function/Dictionary Errors

**"Unknown function sample_project_city_name":**
- Function wasn't created or failed to create
- Function SQL has syntax error
- Function name doesn't match reference
- Missing `__PROJECT_NAME__` template variable

**"Dictionary not found":**
- Dictionary files missing or mismatched names
- Dictionary wasn't uploaded/created
- Wrong dictionary name in SQL query
- Missing project prefix in reference

**Solution:**
```bash
# Check what was created
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

# Clean up and retry
deno run --allow-all src/cleanup.ts --all mcdn_test
deno run --allow-all src/main.ts --local mcdn_test
```

---

### Transform Validation Errors

**"Columns of type double may not be indexed":**
```json
// Fix: Set index to false for double columns
{"type": "double", "index": false}
```

**"Sample data doesn't match schema":**
- Review transform column definitions
- Check sample data types match column types
- Use proper casting in SQL (e.g., `toUInt64(double_value * 1000)`)

---

### Cleanup Issues

**"Failed to delete function/dictionary":**
- Resource might be in use by a transform
- Try deleting tables first, then dependencies
- Use dry-run to see what's blocking deletion

**Accidentally deleted too much:**
- Always specify bundle name: `--all [bundle_name]`
- Use `--dry-run` first to preview
- Keep backups of bundle definitions in git

---

## Best Practices

### 1. Always Validate Before Deploying

```bash
# Fast validation first (30 seconds)
deno run --allow-all src/main.ts mcdn_test

# Then deploy if validation passes
deno run --allow-all src/main.ts --local mcdn_test
```

### 2. Use Template Variables Everywhere

```sql
-- ❌… Correct
__PROJECT_NAME___city_name(ip)

-- ❌ Wrong (hardcoded)
sample_project_city_name(ip)
reference_city_name(ip)
```

### 3. Test Incrementally

```bash
# 1. Validate
deno run --allow-all src/main.ts mcdn_test

# 2. Dashboard only (if just changing visualizations)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# 3. Full test (when changing data/transforms)
deno run --allow-all src/main.ts --local mcdn_test
```

### 4. Always Use Bundle-Scoped Cleanup

```bash
# ❌… Safe - only deletes mcdn_test resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# ❌ Dangerous - deletes EVERYTHING!
deno run --allow-all src/cleanup.ts --all
```

### 5. Use Dry-Run for Safety

```bash
# Preview before deleting
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

# Then delete if output looks correct
deno run --allow-all src/cleanup.ts --all mcdn_test
```

### 6. Commit Often, Test Before Push

```bash
# Before committing
deno run --allow-all src/main.ts mcdn_test

# Before pushing
deno run --allow-all src/main.ts --local mcdn_test
```

---

## Testing Strategy

| Test Level | Command | Speed | Coverage | Use Case |
|------------|---------|-------|----------|----------|
| **Validation** | `main.ts [bundle]` | Fast (30s) | Structure & files | Development, CI/CD |
| **Dashboard Only** | `--local-dashboard-only` | Medium (2min) | Dashboards & alerts | Dashboard iteration |
| **Full Local** | `--local` | Slow (5min) | End-to-end | Final validation, production readiness |
| **Production Check** | `--production` | Medium (60s) | Dependency existence | Pre-deployment validation |

---

## Exit Codes

- **0**: All tests passed successfully
- **1**: Test failure (validation error, deployment failure, or browser errors)

The tool exits immediately on the first failure encountered.

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Validate Bundle
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: denoland/setup-deno@v1
        with:
          deno-version: v1.x
      
      - name: Validate Bundle
        env:
          BUNDLE_TESTING_CLUSTER: ${{ secrets.CLUSTER }}
          BUNDLE_TESTING_USERNAME: ${{ secrets.USERNAME }}
          BUNDLE_TESTING_PASSWORD: ${{ secrets.PASSWORD }}
        run: |
          deno run --allow-all src/main.ts mcdn_test
```

---

## Getting Help

**Documentation:**
- [BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md) - Bundle format specification
- [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) - Validation rules
- [Bundle Deployer.md](./Bundle%20Deployer.md) - Complete user guide
- [Hydrolix Docs](https://docs.hydrolix.io/) - Platform reference

**Common Issues:**
- Review validation output for specific errors
- Check example bundles in `my-bundles/` for patterns
- Use cleanup tool for fresh starts
- Verify environment variables are set correctly