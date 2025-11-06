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

# Optional: Override shared project name (defaults to hdx_solutions)
export SHARED_PROJECT_NAME="hdx_solutions"
```

**Note**: These credentials are used to:
- Authenticate with Hydrolix cluster
- Create test resources (tables, functions, dictionaries)
- Deploy and validate bundles
- Manage shared resources in hdx_solutions project

---

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
- ✅ Function/dictionary file checks (shared and bundle-specific)
- ✅ No deployment or data creation
- ℹ️  Plugin validation skipped (requires deployed dashboards)

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

This performs complete end-to-end testing including:
- All validation checks (same as above)
- Shared resource creation/verification in `hdx_solutions` project
- Bundle-specific resource creation in bundle's project
- Table creation with transforms
- Sample data insertion
- Summary table creation (if defined)
- Grafana datasource and dashboard deployment
- Alert rules creation (if defined)
- Headless Chrome testing (30-second dashboard load)
- Grafana plugin detection and validation (all dashboards)
- Error detection (datasource errors, query errors, missing plugins)

**Example:**
```bash
deno run --allow-all src/main.ts --local mcdn_test
```

**When to use**: Final testing before release, validating end-to-end functionality.

**Time**: ~3-5 minutes (depends on data volume and cluster response time)

**What happens:**
1. Validates bundle structure
2. Extracts `dictionaries.zip` if present
3. Creates/checks `hdx_solutions` project
4. Creates/checks shared functions and dictionaries
5. Creates bundle-specific functions and dictionaries
6. Creates tables and adds transforms
7. Inserts sample data
8. Creates summary tables
9. Starts Grafana container
10. Deploys dashboards and alert rules
11. Tests in headless Chrome
12. Validates Grafana plugins (checks all deployed dashboards)

---

### 3. Dashboard-Only Deployment

```bash
deno run --allow-all src/main.ts --local-dashboard-only [bundle_name]
```

This deploys dashboards without creating tables or data:
- ✅ All validation checks
- ✅ Grafana container setup
- ✅ Datasource creation
- ✅ Dashboard deployment
- ✅ Alert rules creation (if defined)
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

This validates dependencies exist on the target cluster without deploying:
- Checks shared functions exist in hdx_solutions (with `hdx_solutions_` prefix)
- Checks shared dictionaries exist in hdx_solutions (with `hdx_solutions_` prefix)
- Checks bundle-specific functions exist on cluster (with project prefix)
- Checks bundle-specific dictionaries exist on cluster (with project prefix)
- Verifies local definition files are present
- **Does NOT** deploy or create anything

**Example:**
```bash
deno run --allow-all src/main.ts --production mcdn_test
```

**When to use**: Validating bundle readiness for production deployment, CI/CD pre-deployment checks.

**Time**: ~30-60 seconds

**Success output:**
```
❌“ All required dependencies exist on cluster
❌“ All required local files present
```

**Failure output:**
```
❌ Missing functions on cluster (hdx_solutions):
   - city_name (expected as: hdx_solutions_city_name)

❌ Missing dictionaries on cluster (sample_project):
   - custom_dict (expected as: sample_project_custom_dict)
```

---

### 5. Generate Deployment Output

```bash
deno run --allow-all src/main.ts --output [bundle_name]
```

Dumps detailed deployment information in JSON format:
- Cluster domain
- Project name
- Grafana domain and datasource UID
- Dashboard ID
- Table names and transform details

**When to use**: Traffic generation, integration with other tools, debugging deployments.

---

### 6. Cleanup Bundle Resources

```bash
# Delete all bundle-specific resources (SAFE - shared resources preserved)
deno run --allow-all src/cleanup.ts --all [bundle_name]

# Delete specific resource types
deno run --allow-all src/cleanup.ts --functions [bundle_name]
deno run --allow-all src/cleanup.ts --dictionaries [bundle_name]
deno run --allow-all src/cleanup.ts --dictionary-files [bundle_name]
deno run --allow-all src/cleanup.ts --tables [bundle_name]

# Dry run (preview deletion without executing)
deno run --allow-all src/cleanup.ts --all [bundle_name] --dry-run
```

**Examples:**
```bash
# Safe - only deletes mcdn_test bundle-specific resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# Preview what would be deleted
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run
```

**Important Notes**:
- Cleanup only removes **bundle-specific** resources from the bundle's project
- **Shared resources** in `hdx_solutions` are preserved (may be used by other bundles)
- To delete shared resources, manually remove them in Hydrolix UI
- Running without bundle name deletes ALL resources in project (dangerous!)

---

## Command Line Options

| Flag | Description | Time | Use Case |
|------|-------------|------|----------|
| `--local` | Full deployment with testing | ~5min | Final validation, production readiness |
| `--local-dashboard-only` | Dashboard deployment only | ~2min | Dashboard iteration |
| `--strict-plugins` | Fail if Grafana plugins missing | - | CI/CD, pre-release validation |
| `--production` | Validate dependencies exist | ~1min | Pre-deployment validation, CI/CD |
| `--output` | Dump deployment info as JSON | - | Traffic generation, debugging |
| `[bundle_name]` | Filter by bundle name | - | Test specific bundles |

---

## Shared Resources Testing

### Understanding Resource Projects

The Bundle Deployer manages resources in two separate projects:

**hdx_solutions (Shared Project)**
- Common functions and dictionaries used across all bundles
- Resources named: `hdx_solutions_{name}`
- Examples: `hdx_solutions_city_name`, `hdx_solutions_geoip_city`
- Created once, shared by all bundles
- Maintained by shared resources team
- Project created automatically if it doesn't exist

**sample_project (Bundle Project)** 
- Bundle-specific resources unique to each bundle
- Resources named: `{project}_{name}`
- Examples: `sample_project_custom_parser`, `sample_project_ua_cat_dict`
- Created per bundle deployment
- Maintained by bundle owner

### Cross-Project References

Functions and dictionaries can reference resources from any project. A bundle-specific function can call a shared dictionary, and vice versa:

```sql
-- Bundle-specific function calling shared dictionary (valid!)
CREATE FUNCTION sample_project_enrich AS
(ip) -> dictGet('hdx_solutions_geoip_city', 'city_name', ip)
```

### Testing Scenarios

**Scenario 1: First Deployment (Creates Shared Resources)**

```bash
# Clean slate - manually delete hdx_solutions in Hydrolix UI if testing fresh
deno run --allow-all src/cleanup.ts --all mcdn_test

# Deploy - creates hdx_solutions project and all shared resources
deno run --allow-all src/main.ts --local mcdn_test
```

**Expected output:**
```
🔍— Checking for shared project: hdx_solutions...
  Creating shared project: hdx_solutions...
  ❌“ Created shared project (uuid: 90943f33-66df-430d-83e8-18fd2e218e49)

🔍— Processing 4 EXPLICITLY DECLARED shared function(s) in hdx_solutions...
Checking shared function: city_name...
  Creating shared function city_name (will become hdx_solutions_city_name)...
  ❌“ Created shared function city_name
Checking shared function: breadcrumbs...
  Creating shared function breadcrumbs (will become hdx_solutions_breadcrumbs)...
  ❌“ Created shared function breadcrumbs
  [continues for all 4 functions]

🔍— Processing 5 EXPLICITLY DECLARED shared dictionar(y/ies) in hdx_solutions...
Checking shared dictionary: geoip_asn_blocks_ipv4...
  Found files: dictionaries/.extracted/geoip_asn_blocks_ipv4.json + ...csv
  Uploading shared dictionary file: geoip_asn_blocks_ipv4.csv...
  ❌“ Uploaded shared dictionary file
  Creating shared dictionary definition...
  ❌“ Created shared dictionary geoip_asn_blocks_ipv4
  [continues for all 5 dictionaries]
```

**Scenario 2: Second Deployment (Reuses Shared Resources)**

```bash
# Deploy again without cleanup
deno run --allow-all src/main.ts --local mcdn_test
```

**Expected output:**
```
🔍— Checking for shared project: hdx_solutions...
  ❌“ Shared project exists (uuid: 90943f33-66df-430d-83e8-18fd2e218e49)

🔍— Processing 4 EXPLICITLY DECLARED shared function(s) in hdx_solutions...
Checking shared function: city_name...
  ❌“ Shared function city_name exists (as hdx_solutions_city_name)
Checking shared function: breadcrumbs...
  ❌“ Shared function breadcrumbs exists (as hdx_solutions_breadcrumbs)
  [all show "exists" - no creation]

📦 No bundle-specific functions declared (using 4 shared function(s))

📦 Processing 1 EXPLICITLY DECLARED bundle-specific dictionar(y/ies) in sample_project...
  [creates bundle-specific resources]
```

Notice: Shared resources are **reused** (logged as "exists"), not recreated. This is idempotent behavior.

**Scenario 3: Different Bundle Using Same Shared Resources**

```bash
# Deploy a second bundle that also declares city_name as shared
deno run --allow-all src/main.ts --local cloudfront_logs
```

**Expected output:**
```
🔍— Checking for shared project: hdx_solutions...
  ❌“ Shared project exists

🔍— Processing 2 EXPLICITLY DECLARED shared function(s) in hdx_solutions...
  ❌“ Shared function city_name exists (as hdx_solutions_city_name)  â† Reused!
  ❌“ Shared function country_iso_code exists  â† Reused!
```

The second bundle reuses shared resources created by the first bundle.

---

## What Gets Validated and Tested

### Validation Phase (Always Run)

#### Structural Validation
- ✅ Bundle JSON structure and required fields
- ✅ URL format validation (`https://` or `file://`)
- ✅ Path validation (no `/`, no `..`, proper extensions)
- ✅ Macro variable format (`__VARIABLE_NAME__`)
- ✅ Enum validation (method, source, channel_type, data_category)
- ✅ SHA256 checksum format (64 hex characters)
- ✅ Bundle name format (alphanumeric, underscores, dashes)
- ✅ Table name validation (â‰¥3 chars, starts with letter, alphanumeric + underscore)

#### Content Validation
- ✅ No duplicate table names or dashboard variables within bundle
- ✅ Naming consistency (method titles align with method, source titles unique)
- ✅ File existence and accessibility for all referenced files
- ✅ JSON syntax validation for all JSON files
- ✅ Transform structure (name field, valid JSON)
- ✅ Sample data presence (non-empty object or string)
- ✅ Dashboard structure (top-level dashboard object, no hardcoded id)
- ✅ Alert rules structure (apiVersion, groups, rules with required fields)
- ✅ Summary table references (parent_table_name must exist)

#### Shared Resources Validation
- ✅ Declared shared functions have local files (`functions/{name}.json`)
- ✅ Declared shared dictionaries have both files (`.json` + data file)
- ✅ Declared bundle-specific functions have local files
- ✅ Declared bundle-specific dictionaries have both files
- ✅ SQL references match declared functions/dictionaries
- ✅ Template variables used correctly (`__SHARED_PROJECT__` vs `__PROJECT_NAME__`)
- ⚠️ Warns if declared resources unused in SQL
- ⚠️ Warns if SQL uses undeclared resources

#### Cross-Bundle Validation
- ✅ No duplicate bundle names globally
- ✅ No duplicate UI source titles globally
- ✅ No duplicate table names globally
- ✅ No duplicate base URLs globally

---

### Deployment Phase (With `--local`)

#### Shared Resources Management
**hdx_solutions Project**:
- Checks if `hdx_solutions` project exists via API
- Creates project if missing (first deployment ever)
- Caches project UUID in memory for subsequent operations

**Shared Functions**:
- Lists existing functions in hdx_solutions project
- Checks if each declared shared function exists (matches by name without prefix)
- Skips if function exists (logs "❌“ exists")
- Creates from local file if missing:
  - Reads `functions/{name}.json`
  - Replaces `__SHARED_PROJECT__` and `__PROJECT_NAME__` template variables
  - POSTs to hdx_solutions functions endpoint
  - Function deployed as: `hdx_solutions_{name}`
- **Fails immediately** if declared shared function has no local file

**Shared Dictionaries**:
- Lists existing dictionaries in hdx_solutions project
- Checks if each declared shared dictionary exists (matches by name without prefix)
- Skips if dictionary exists (logs "❌“ exists")
- Creates from local files if missing:
  - Searches for files in `dictionaries/` then `dictionaries/.extracted/`
  - Reads definition (`{name}.json`) and data file (`{name}.[csv/yaml/yml/tsv]`)
  - Uploads data file to hdx_solutions
  - Creates dictionary definition
  - Dictionary deployed as: `hdx_solutions_{name}`
- **Fails immediately** if declared shared dictionary missing files

#### Bundle-Specific Resources
**Zip Extraction**:
- Automatically detects and extracts `dictionaries/dictionaries.zip`
- Extracts to `dictionaries/.extracted/` (gitignored)
- Flattens directory structure using `-j` flag
- Skips if already extracted

**Auto-Discovery** (when not declared):
- **Functions**: Scans `functions/` for all `.json` files
- **Dictionaries**: Scans `dictionaries/` and `.extracted/` for matching `.json` + data file pairs
- Only triggers if BOTH `required_*` AND `shared_*` are omitted for that resource type
- All discovered resources treated as bundle-specific (never shared)

**Resource Creation**:
- Functions created in bundle's project as `{project}_{name}`
- Dictionaries uploaded and created in bundle's project as `{project}_{name}`
- Template variables (`__PROJECT_NAME__`) replaced during creation
- **Fails immediately** if declared resource missing local files

#### Data Pipeline
**Tables**: Created with 1-day retention, waits 30 seconds for readiness

**Transforms**: 
- Attached to tables via API
- SQL transformed with both `__PROJECT_NAME__` and `__SHARED_PROJECT__` replacement
- Retry logic: up to 5 attempts with exponential backoff (1s â†’ 30s max delay)
- Validates transform against sample data

**Sample Data**:
- Inserted into tables for testing
- Retry logic: up to 20 attempts with exponential backoff (1s â†’ 60s max delay)
- Waits 30 seconds for table readiness before insertion
- **Warns if insertion fails** but continues deployment (dashboard created without data)

**Summary Tables**: Created from SQL files with template variable replacement (`__PROJECT_NAME__`, `__SHARED_PROJECT__`, `__TABLE_NAME__`)

#### Grafana Deployment
**Container Management**:
- Kills existing Grafana container
- Starts fresh container
- Waits for health check (60-second timeout)

**Datasource**: Creates Hydrolix datasource pointing to bundle's project

**Dashboards**:
- Loads dashboard JSON and replaces template variables:
  - `__PROJECT_NAME__` â†’ bundle project name
  - `__SHARED_PROJECT__` â†’ `hdx_solutions`
  - `__DATASOURCE__` â†’ datasource UID
  - `__DASHBOARD_UUID__` â†’ unique ID
  - Table `dashboard_var` â†’ full table names
- Deploys primary dashboard
- Deploys additional dashboards from `other_dashboards` array (if present)

**Alert Rules** (if defined):
- Creates Grafana folders for rule groups
- Deploys individual rules via API
- Removes UI-only fields before submission
- Replaces same template variables as dashboards

#### Browser Testing
- Authenticates with Grafana (gets session cookie)
- Launches headless Chrome
- Navigates to deployed dashboard
- Waits 30 seconds for all panels to load and query data
- Monitors console for errors:
  - Datasource errors: `Datasource \w+ was not found`
  - Query errors: `400 \w+`
- Reports error counts
- **Deployment fails if any errors detected**

#### Plugin Validation (After Deployment)
- Queries all deployed dashboards via Grafana API
- Extracts panel types from dashboard JSON
- Checks installed plugins via `/api/plugins`
- Identifies missing external plugins
- **Default**: Warns about missing plugins but continues
- **With `--strict-plugins`**: Fails deployment if plugins missing
- Reports which dashboards and panels need each plugin

---

## Common Test Scenarios

### During Development

```bash
# Quick validation (30 seconds)
deno run --allow-all src/main.ts mcdn_test

# Dashboard iteration (2 minutes)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Full end-to-end test (5 minutes)
deno run --allow-all src/main.ts --local mcdn_test
```

### Before Committing

```bash
# Validate structure
deno run --allow-all src/main.ts mcdn_test

# Full test if changing transforms/data/functions
deno run --allow-all src/main.ts --local mcdn_test

# Strict validation (fails if plugins missing)
deno run --allow-all src/main.ts --local --strict-plugins mcdn_test
```
deno run --allow-all src/main.ts --local mcdn_test
```

### Testing All Bundles

```bash
# Validate all (fast)
deno run --allow-all src/main.ts

# Full test all (slow - use with caution)
deno run --allow-all src/main.ts --local
```

### Testing Shared Resources

```bash
# First bundle - creates shared resources
deno run --allow-all src/main.ts --local mcdn_test

# Second bundle - reuses shared resources
deno run --allow-all src/main.ts --local cloudfront_logs

# Verify in Hydrolix UI:
# - hdx_solutions: shared functions and dictionaries
# - sample_project: bundle-specific resources only
```

### Testing Auto-Discovery

```bash
# Bundle with empty dependencies - auto-discovers everything
deno run --allow-all src/main.ts --local vendor_bundle

# Expected: All resources created as bundle-specific in sample_project
```

### Iterating on Dashboards

```bash
# Deploy dashboard
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Make changes to dashboard JSON

# Redeploy (Grafana restarts automatically)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test
```

### Testing with Fresh Environment

```bash
# Clean bundle-specific resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# Manually delete hdx_solutions in Hydrolix UI (if testing first-time creation)

# Deploy fresh
deno run --allow-all src/main.ts --local mcdn_test
```

### Debugging Failed Tests

```bash
# Get detailed output
deno run --allow-all src/main.ts --output --local mcdn_test

# Validate only
deno run --allow-all src/main.ts mcdn_test

# Preview cleanup
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

# Clean and retry
deno run --allow-all src/cleanup.ts --all mcdn_test
deno run --allow-all src/main.ts --local mcdn_test
```

---

## Understanding Test Results

### Validation Success
```
Testing http_streaming_mcdn_test
❌“ All required dependencies exist on cluster
❌“ All required local files present
Final check on all bundles for duplicated tokens...
SUCCESS
```

### Validation Failure
```
ERROR: Failed bundle validation: Transform file is not valid JSON: path=transformations/mcdn_akamai.json
```

### Deployment Success
```
🔍— Processing 4 shared function(s) in hdx_solutions...
  ❌“ Shared function city_name exists
  ❌“ Shared function breadcrumbs exists
  [...]

📦 No bundle-specific functions declared (using 4 shared function(s))

Creating table: mcdn_test
❌“ Successfully inserted sample data
❌“ Created Grafana datasource
Starting headless browser test...
Datasource errors: 0
SUCCESS
```

### Deployment Failure
```
❌ Transform validation failed:
   Error: Unknown function sample_project_city_name

ERROR: Failed to add transform
```

### Browser Test Failure
```
ERROR: Datasource not found - Datasource Hydrolix was not found
Datasource errors: 2
Dashboard Errors=2
```

---

## Directory Structure

Your bundle must follow this structure:

```
my-bundles/mcdn_test/
├── bundle.json                    # Required: Bundle manifest
├── functions/                      # Optional: SQL functions
│   ├── city_name.json             # Shared or bundle-specific
│   └── breadcrumbs.json
├── dictionaries/                   # Optional: Lookup tables
│   ├── dictionaries.zip           # Large files (auto-extracted)
│   ├── .extracted/                # Auto-created (gitignored)
│   ├── ua_cat_dict.json           # Definition
│   └── ua_cat_dict.yaml           # Data
├── transformations/                # Required: Data schemas
│   ├── mcdn_akamai.json
│   └── mcdn_cloudflare.json
├── dashboards/                     # Required: Visualizations
│   ├── CDN Dashboard.json
│   ├── alert-rules.json           # Optional
│   └── Raw Logs.json              # Optional
└── summaries/                      # Optional: Pre-aggregations
    └── mcdn_summary_min.sql
```

---

## Troubleshooting

### Environment Variables

```bash
# Check if set
echo $BUNDLE_TESTING_CLUSTER
echo $BUNDLE_TESTING_USERNAME
echo $BUNDLE_TESTING_PASSWORD

# Set if missing
export BUNDLE_TESTING_CLUSTER="partnersandbox.trafficpeak.live"
export BUNDLE_TESTING_USERNAME="your-username"
export BUNDLE_TESTING_PASSWORD="your-password"
```

### Docker Issues

```bash
# Check Docker running
docker ps

# Manual Grafana cleanup if needed
docker ps -a | grep grafana
docker stop <container-id>
```

### Browser Testing Failures

**Datasource errors:** Verify datasource created, check cluster accessibility, confirm credentials

**Dashboard not loading:** Check Grafana running (`docker ps`), review dashboard JSON for template issues

**Chrome issues:** Ensure Chrome/Chromium installed, set `PUPPETEER_EXECUTABLE_PATH` if needed

### Plugin Validation Issues

**"Missing plugins detected" (Warning):**
```
⚠️  WARNING: Missing plugins detected!
  • marcusolsson-treemap-panel - 1 panel(s) across 1 dashboard(s)
```

**Solution:** Update `src/grafana/container.ts` to install plugins:
```typescript
const cmd = new Deno.Command("docker", {
  args: [
    "run", "--rm", "-d", "-p", "3000:3000",
    "-e", "GF_INSTALL_PLUGINS=marcusolsson-treemap-panel",
    "javiani/grafana:latest"
  ],
});
```

**Plugin validation fails with `--strict-plugins`:**
```
❌ ERROR: Missing required Grafana plugins!
Plugin validation failed: 1 required plugin(s) missing
```

**Solution:** Install required plugins before using `--strict-plugins` flag

**Plugin check shows only primary dashboard:**
- Ensure `deploy.ts` and `deploy_only_dashboard.ts` return array of UIDs
- Verify all dashboard UIDs passed to `checkDeployedDashboards()`

### Shared Resources Issues

**"Shared function declared but file not found":**
```
❌ Shared function 'city_name' declared but file not found.
   Expected: my-bundles/mcdn_test/functions/city_name.json
```

**Solution:** 
- Add `functions/city_name.json`, OR
- Remove `city_name` from `shared_functions` in bundle.json, OR
- Check spelling/capitalization

**"Auto-discovering even though shared declared":**
```
📦 AUTO-DISCOVERING bundle-specific functions...
  Found: city_name  â† Should be shared!
```

**Cause:** Empty array in bundle.json:
```json
"required_functions": [],  // ❌ Triggers auto-discovery
"shared_functions": ["city_name"]
```

**Solution:** Remove empty array:
```json
// ✅ Just omit required_functions
"shared_functions": ["city_name"]
```

**"Functions created in wrong project":**
```
❌“ Created function city_name  // In sample_project, should be hdx_solutions!
```

**Solution:**
- Verify `shared_functions` array in bundle.json
- Ensure no empty `required_functions: []`
- Check helper functions imported in deploy.ts

**"Duplicate functions in both projects":**

**Solution:**
1. Delete bundle-specific duplicates:
   ```bash
   deno run --allow-all src/cleanup.ts --functions mcdn_test
   ```
2. Update bundle.json to declare as shared
3. Redeploy

### Function/Dictionary Errors

**"Unknown function hdx_solutions_city_name":**
- Missing correct template variable (`__SHARED_PROJECT__` for shared, `__PROJECT_NAME__` for bundle-specific)
- Function not created or creation failed

**"Dictionary not found":**
- Missing files (need `.json` + data file)
- Wrong template variable for resource type

**Solution:** Clean and retry:
```bash
deno run --allow-all src/cleanup.ts --all mcdn_test
deno run --allow-all src/main.ts --local mcdn_test
```

### Transform/Cleanup Errors

**"Columns of type double may not be indexed":** Set `{"type": "double", "index": false}`

**"Sample data doesn't match schema":** Review column definitions, use proper casting

**"Failed to delete resource":** May be in use - delete tables first, use `--dry-run` to preview

---

## Best Practices

### 1. Always Validate Before Deploying

```bash
# Fast validation first
deno run --allow-all src/main.ts mcdn_test

# Deploy if passes
deno run --allow-all src/main.ts --local mcdn_test
```

### 2. Use Correct Template Variables

```sql
-- ✅ Shared resources
SELECT __SHARED_PROJECT___city_name(ip) AS city
FROM dictGet('__SHARED_PROJECT___geoip_city', 'city_name', ip)

-- ✅ Bundle-specific resources
SELECT __PROJECT_NAME___custom_parser(data) AS parsed
FROM dictGet('__PROJECT_NAME___custom_dict', 'key', value)

-- ❌ Wrong (hardcoded)
SELECT sample_project_city_name(ip)
SELECT reference_city_name(ip)
```

### 3. Declare Shared Resources Explicitly

For resources used by multiple bundles:
```json
{
  "shared_functions": ["city_name", "breadcrumbs"],
  "shared_dictionaries": ["geoip_city"]
}
```

**Benefits**: Clear documentation, proper categorization, no duplicates, easier maintenance

### 4. Don't Use Empty Arrays

```json
❌ "required_functions": []  // Disables auto-discovery

✅ // Omit field entirely to enable auto-discovery
```

### 5. Test Incrementally

```bash
# 1. Validate structure
deno run --allow-all src/main.ts mcdn_test

# 2. Dashboard only (for visualization changes)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# 3. Full test (for data/transform changes)
deno run --allow-all src/main.ts --local mcdn_test
```

### 6. Use Bundle-Scoped Cleanup

```bash
# ✅ Safe - only mcdn_test resources
deno run --allow-all src/cleanup.ts --all mcdn_test

# ❌ Dangerous - deletes EVERYTHING
deno run --allow-all src/cleanup.ts --all
```

### 7. Use Dry-Run for Safety

```bash
# Preview before deleting
deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

# Execute if looks correct
deno run --allow-all src/cleanup.ts --all mcdn_test
```

---

## Resource Declaration Modes

### Explicit Mode (Recommended)
```json
{
  "dependencies": {
    "hydrolix": {
      "shared_functions": ["city_name"],
      "shared_dictionaries": ["geoip_city"],
      "required_dictionaries": ["custom_dict"]
    }
  }
}
```
- Clear documentation, validates files exist, no surprises

### Auto-Discovery Mode (Vendor Bundles)
```json
{
  "dependencies": {"hydrolix": {}}
}
```
- Quick start, zero config
- All discovered resources treated as bundle-specific
- No shared resources created

### Hybrid Mode (Best Flexibility)
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
- Shared explicit and validated, bundle-specific auto-discovered

---

## Testing Strategy

| Test Level | Command | Speed | Coverage |
|------------|---------|-------|----------|
| **Validation** | `main.ts [bundle]` | 30s | Structure & files |
| **Dashboard Only** | `--local-dashboard-only` | 2min | Dashboards & alerts |
| **Full Local** | `--local` | 5min | End-to-end |
| **Production Check** | `--production` | 1min | Dependency existence |

---

## Exit Codes

- **0**: Success
- **1**: Failure (exits on first error)

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
      
      - name: Validate
        env:
          BUNDLE_TESTING_CLUSTER: ${{ secrets.CLUSTER }}
          BUNDLE_TESTING_USERNAME: ${{ secrets.USERNAME }}
          BUNDLE_TESTING_PASSWORD: ${{ secrets.PASSWORD }}
        run: deno run --allow-all src/main.ts mcdn_test
      
      - name: Production Check
        env:
          BUNDLE_TESTING_CLUSTER: ${{ secrets.PROD_CLUSTER }}
          BUNDLE_TESTING_USERNAME: ${{ secrets.PROD_USERNAME }}
          BUNDLE_TESTING_PASSWORD: ${{ secrets.PROD_PASSWORD }}
        run: deno run --allow-all src/main.ts --production mcdn_test
```

---

## Getting Help

**Documentation:**
- [BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md) - Bundle format specification
- [WHAT-IS-CHECKED.md](./WHAT-IS-CHECKED.md) - Validation rules
- [Bundle Deployer.md](./Bundle%20Deployer.md) - User guide
- [Hydrolix Docs](https://docs.hydrolix.io/)

**For errors:**
- Read error message and referenced file/line
- Review validation rules
- Check example bundles
- Use cleanup tool with --dry-run
- Verify resources in correct project
- Ensure correct template variables