# Validation & Testing Coverage

This document outlines the comprehensive validation and testing performed by the TypeScript/Deno Bundle Deployer. The system ensures that integration bundles meet all quality, consistency, and functional requirements before and during deployment.

## Overview

The Bundle Deployer performs validation and testing in multiple phases:

1. **Individual Bundle Validation** - Each bundle is validated independently
2. **Global Cross-Bundle Validation** - All bundles are validated together for conflicts
3. **Integration Testing** (with `--local`) - Functional testing with live Hydrolix and Grafana
4. **Browser Testing** (with `--local` or `--local-dashboard-only`) - Headless Chrome validation
5. **Plugin Validation** (with `--local` or `--local-dashboard-only`) - Runtime Grafana plugin detection

---

## Validation Phase (Always Runs)

These checks run for every command, regardless of flags:

### 1. Base URL Validation (`valid_base_url.ts`)

**Purpose**: Ensures bundle references the correct GitHub repository location.

**Checks**:
- âœ… `bundle.base_url` matches expected format
- âœ… Path includes bundle directory name
- âœ… Points to main branch

**Expected format**: `https://github.com/hydrolix/integration-deployment-templates/blob/main/my-bundles/{bundle_name}`

**Failure example**: `Invalid base_url should be: 'https://github.com/hydrolix/.../my-bundles/mcdn_test'`

---

### 2. Naming Convention Validation (`naming_is_valid.ts`)

**Purpose**: Enforces consistent naming conventions across bundle components.

**Checks**:
- âœ… Method-title consistency (e.g., `firehose` â†’ UI title contains "AWS Firehose")
- âœ… Source-title consistency (e.g., `waf` source â†’ UI title contains "WAF")
- âœ… Bundle name includes both source and method (case-insensitive)
- âœ… Semantic versioning format (X.Y.Z with exactly 2 dots)
- âœ… Valid maintainer email (contains `@` and `.`)
- âœ… Non-empty description

**Failure examples**:
- `docs.method.full_title 'Simple S3' does not match method 'firehose'`
- `Name 'simple' must include 'cloudfront' and 'kinesis'`
- `Version 1.0 should follow semantic versioning format`

---

### 3. Duplicate Token Validation (`no_duplicate_tokens.ts`)

**Purpose**: Prevents naming conflicts within a single bundle.

**Checks**:
- âœ… No duplicate table names
- âœ… Table names â‰¥3 characters, start with letter, alphanumeric + underscore only
- âœ… No duplicate dashboard variables

**Failure examples**:
- `Duplicate table name mcdn_test`
- `Invalid table name '123table' - must start with a letter`
- `Invalid table name 'table-name' - only letters, digits, and underscores allowed`

---

### 4. Checksum Validation (`no_bad_checksums.ts`)

**Purpose**: Ensures file integrity through SHA256 checksum verification.

**Checks**:
- âœ… Dashboard file checksums (if provided)
- âœ… Transform file checksums (if provided)
- âœ… Summary SQL file checksums (if provided)
- âœ… Computed checksums match declared checksums

**Failure example**: `SHA256 abc123...def does not match for local file transformations/mcdn_akamai.json`

**Note**: Checksums are optional. If not provided, validation is skipped.

---

### 5. Transform File Validation (`transforms_are_valid.ts`)

**Purpose**: Validates all transformation files referenced in the bundle.

**Checks**:
- âœ… All transform files exist and readable
- âœ… Valid JSON syntax
- âœ… Required `name` field (non-empty string)
- âœ… No duplicate transform names within table
- âœ… `subtype` must be "firehose" if present

**Failure examples**:
- `Transform file is not valid JSON: path=... error=Unexpected token`
- `Transform file missing required 'name' field`
- `Duplicated transform name 'mcdn_akamai'`

---

### 6. Sample Data Validation (`sample_data_exists.ts`)

**Purpose**: Ensures all transforms include sample data for testing.

**Checks**:
- âœ… Each transform contains `settings.sample_data`
- âœ… Sample data is non-empty object or string

**Failure example**: `No Sample data in transformation full_path=transformations/mcdn_akamai.json`

---

### 7. Dashboard Validation (`dashboard_is_valid.ts`)

**Purpose**: Validates Grafana dashboard files and their template variables.

**Checks**:
- âœ… File exists, valid JSON
- âœ… Required template variables: `__DASHBOARD_UUID__`, `__DATASOURCE__`, `__PROJECT_NAME__`, all table `dashboard_var` values
- âœ… Top-level `dashboard` object exists
- âœ… No hardcoded `id` field
- âœ… Validates primary dashboard and `other_dashboards` array

**Failure examples**:
- `Dashboard must have __DATASOURCE__`
- `Invalid dashboard - top element must be dashboard`
- `Invalid dashboard - cannot have Id set`

---

### 8. Alert Rules Validation (`alert_rules_are_valid.ts`)

**Purpose**: Validates alert rules file structure (if present).

**Checks**:
- âœ… Valid JSON with `apiVersion` field
- âœ… `groups` array with â‰¥1 group
- âœ… Each group has: `name`, `folder`, `interval`, `rules` (â‰¥1 rule)
- âœ… Each rule has: `uid`, `title`, `condition`, `data` (array)

**Failure example**: `Group alerts, Rule 0: uid is required`

**Note**: Alert rules are optional. Validation skipped if not defined.

---

### 9. Summary Table Validation (`summary_table.ts`)

**Purpose**: Validates summary table references and prevents duplicates.

**Checks**:
- âœ… `parent_table_name` references valid table in bundle
- âœ… No duplicate `dashboard_var` values
- âœ… Unique summary table names

**Failure example**: `Invalid-Parent-Table-Reference summary_table=mcdn_summary_min parent_table_name=nonexistent`

**Note**: Summary tables are optional. Validation skipped if not defined.

---

### 10. Dependency Validation (`check_dependencies.ts`)

**Purpose**: Validates function and dictionary files exist and SQL references match.

**Checks**:
- âœ… Declared shared/bundle-specific functions have local files (`functions/{name}.json`)
- âœ… Declared shared/bundle-specific dictionaries have both files (`.json` + data)
- âœ… SQL references match declared functions (scans for `functionName\s*\(` pattern)
- âœ… SQL dictionary calls match declarations (scans for `dictGet`, `dictGetString`, `dictGetOrDefault`)
- âš ï¸ Warns if declared but unused
- âš ï¸ Warns if used but undeclared

**Warning examples**:
- `Function 'city_name' declared but no file: functions/city_name.json`
- `Dictionary 'ua_cat_dict' has definition but no data file`
- `Transform uses dictionary 'geoip_dict' but not declared`
- `Function 'unused_func' declared but not used`

**Note**: Produces warnings only, does not fail validation.

---

## Global Cross-Bundle Validation

### 11. Global Duplicate Prevention (`no_global_duplicates.ts`)

**Purpose**: Prevents conflicts across all bundles in the repository.

**Checks**:
- âœ… No duplicate bundle names
- âœ… No duplicate UI source titles
- âœ… No duplicate table names
- âœ… No duplicate base URLs

**Failure examples**:
- `Duplicated-Bundle-Name error=mcdn_test`
- `Duplicated-UI-Source-Name error=MCDN TEST`
- `Duplicated-Name count=2 table=mcdn_test`

---

## Integration Testing Phase (With `--local` flag)

These tests run only when deploying locally:

### 12. Shared Resources Management

**hdx_solutions Project** (`hdx_shared.ts::ensureSharedProjectExists`):
- Checks if `hdx_solutions` project exists via API
- Creates project if missing (first deployment ever)
- Caches project UUID in memory for subsequent operations
- Handles paginated API responses (extracts from `results`, `projects`, or `data` arrays)

**Shared Functions** (`hdx_shared.ts::checkAndCreateSharedFunction`):
- Lists existing functions in hdx_solutions project
- Checks if function exists by name (API returns name without project prefix)
- Skips if exists (logs "âœ“ exists")
- Creates from local file if missing:
  - Reads `functions/{name}.json`
  - Replaces `__SHARED_PROJECT__` and `__PROJECT_NAME__` template variables
  - POSTs to hdx_solutions functions endpoint
  - Deployed as: `hdx_solutions_{name}`
- **Fails immediately** if declared shared function has no local file

**Shared Dictionaries** (`hdx_shared.ts::checkAndCreateSharedDictionary`):
- Lists existing dictionaries in hdx_solutions project
- Checks if dictionary exists by name (API returns name without project prefix)
- Skips if exists (logs "âœ“ exists")
- Creates from local files if missing:
  - Searches `dictionaries/` then `.extracted/` for files
  - Uploads data file to hdx_solutions
  - Creates dictionary definition
  - Deployed as: `hdx_solutions_{name}`
- **Fails immediately** if declared shared dictionary missing files

**Output example (first deployment)**:
```
ðŸ”— Creating shared project: hdx_solutions...
  âœ“ Created shared project (uuid: xxx)

ðŸ”— Processing 4 shared function(s) in hdx_solutions...
  Creating shared function city_name...
  âœ“ Created shared function city_name
```

**Output example (subsequent deployment)**:
```
ðŸ”— Checking for shared project: hdx_solutions...
  âœ“ Shared project exists (uuid: xxx)

ðŸ”— Processing 4 shared function(s) in hdx_solutions...
  âœ“ Shared function city_name exists (as hdx_solutions_city_name)
```

---

### 13. Bundle-Specific Resources

**Zip Extraction** (`hdx.ts::ensureZipExtracted`):
- Detects `dictionaries/dictionaries.zip`
- Extracts to `.extracted/` directory using `unzip -j` (flattens structure)
- Skips if already extracted

**Auto-Discovery** (`hdx.ts::discoverFunctions`, `hdx.ts::discoverDictionaries`):
- **Functions**: Scans `functions/` for `.json` files
- **Dictionaries**: Scans `dictionaries/` and `.extracted/` for matching `.json` + data file pairs
- Only triggers if BOTH `required_*` AND `shared_*` omitted for that resource type
- All discovered resources treated as bundle-specific (never shared)

**Auto-discovery logic**:
```typescript
if (bundleFuncs.length === 0 && sharedFuncs.length === 0) {
  // Both empty - auto-discover as bundle-specific
} else if (bundleFuncs.length === 0) {
  // Bundle empty but shared exists - no bundle-specific
  functionsToCreate = [];
}
```

**Bundle-Specific Functions** (`hdx.ts::checkAndCreateFunction`):
- Checks if exists on cluster
- Creates from local file if missing
- Replaces `__PROJECT_NAME__` template variables
- Deployed as: `{project}_{name}`
- **Fails if** declared function has no local file

**Bundle-Specific Dictionaries** (`hdx.ts::checkAndCreateDictionary`):
- Checks if exists on cluster
- Uploads data file to bundle's project
- Creates dictionary definition
- Deployed as: `{project}_{name}`
- **Fails if** declared dictionary missing files

**Output example**:
```
ðŸ“¦ Processing 1 bundle-specific dictionar(y/ies) in sample_project...
  Uploading dictionary file: ua_cat_dict.yaml...
  âœ“ Created dictionary ua_cat_dict
```

---

### 14. Data Pipeline

**Table Creation** (`hdx.ts::createTable`):
- Creates tables with 1-day retention
- Waits 30 seconds for table readiness

**Transform Deployment** (`hdx.ts::addTransformToTable`):
- Replaces `__PROJECT_NAME__` and `__SHARED_PROJECT__` in SQL
- Adds transforms to tables via API
- Retry logic: 5 attempts, exponential backoff (1s â†’ 30s)
- Validates transform against sample data

**Sample Data Insertion** (`hdx.ts::insertIntoTable`):
- Waits 30 seconds for table readiness
- Inserts sample data for testing
- Retry logic: 20 attempts, exponential backoff (1s â†’ 60s)
- Retryable errors: 5xx, 408, 429
- **Warns if fails** but continues (dashboard created without data)

**Summary Tables** (`hdx.ts::createSummaryTable`):
- Loads SQL from file
- Replaces `__PROJECT_NAME__`, `__SHARED_PROJECT__`, `__TABLE_NAME__`
- Creates summary table via API

**Template replacement example**:
```sql
-- Input:
SELECT __SHARED_PROJECT___city_name(ip), COUNT(*) 
FROM __PROJECT_NAME__.__TABLE_NAME__

-- Output:
SELECT hdx_solutions_city_name(ip), COUNT(*) 
FROM sample_project.mcdn_test
```

---

### 15. Grafana Deployment

**Container Management** (`grafana/container.ts`):
- Kills existing Grafana container
- Starts fresh container
- Waits for health check (60s timeout)

**Datasource** (`grafana/interface.ts::createDatalink`):
- Creates Hydrolix datasource pointing to bundle's project
- Returns datasource UID for dashboard references

**Dashboards** (`grafana/interface.ts::createDashboard`):
- Loads dashboard JSON
- Replaces template variables:
  - `__PROJECT_NAME__` â†’ bundle project name
  - `__SHARED_PROJECT__` â†’ `hdx_solutions`
  - `__DATASOURCE__` â†’ datasource UID
  - `__DASHBOARD_UUID__` â†’ unique ID
  - Table/summary table `dashboard_var` â†’ full names
- Deploys primary dashboard and `other_dashboards` array

**Alert Rules** (`grafana/interface.ts::createAlertRules`):
- Creates Grafana folders for rule groups
- Deploys individual rules via API
- Removes UI-only fields before submission
- Replaces template variables

**Output example**:
```
âœ“ Created Grafana datasource with UID: abc123
âœ“ Created primary dashboard (UID: xyz789)
Creating 2 alert rule group(s)...
  âœ“ Created folder "CDN Alerts"
  âœ“ Created rule "High Error Rate"
```

---

### 16. Browser Testing

**Headless Browser Testing** (`headless_browser.ts`):
- Authenticates with Grafana (gets session cookie)
- Launches headless Chrome (Puppeteer)
- Sets viewport (1920x4080)
- Navigates to dashboard
- Monitors console for errors:
  - Datasource errors: `Datasource \w+ was not found`
  - Query errors: `400 \w+`
- Waits 30 seconds for all panels to render
- Reports error counts
- **Deployment fails if errors detected**

**Success output**:
```
Starting headless browser test for dashboard: xyz789
Page loaded - Title: "CDN Dashboard"
Datasource errors: 0
Success!
```

**Failure output**:
```
ERROR: Datasource not found - Datasource Hydrolix was not found
Datasource errors: 2
Dashboard Errors=2
```

---

### 17. Grafana Plugin Validation

**Plugin Detection** (`grafana/grafana_plugins_check.ts`):
- Queries all deployed dashboards via Grafana API (`/api/dashboards/uid/{uid}`)
- Extracts panel types from live dashboard JSON
- Queries installed plugins (`/api/plugins`)
- Identifies external plugins vs built-in panels
- Compares required plugins against installed plugins
- Reports missing plugins with installation instructions
- **Default**: Warns but continues deployment
- **With `--strict-plugins`**: Fails deployment if plugins missing

**Runs**: Only with `--local` or `--local-dashboard-only` flags

**Success output**:
```
🔌 Checking deployed dashboards for plugin usage...
  ✓ Dashboards only use built-in panels
```

**Warning output** (default mode):
```
⚠️  WARNING: Missing plugins detected!

Missing plugins:
  • marcusolsson-treemap-panel - 1 panel(s) across 1 dashboard(s)
    Used in:
      - "CDN Global View" (1 panel(s))

📋 To fix:
  Update grafana/container.ts:
  "-e", "GF_INSTALL_PLUGINS=marcusolsson-treemap-panel"
```

**Error output** (`--strict-plugins` mode):
```
❌ ERROR: Missing required Grafana plugins!

Missing plugins:
  • marcusolsson-treemap-panel

ERROR: Plugin validation failed: 1 required plugin(s) missing
[Exit code 1]
```

**Installed plugins output**:
```
✓ Using 1 external plugin(s) (installed):
  • Treemap Panel (marcusolsson-treemap-panel) - 3 panel(s) across 2 dashboard(s)
```

---

## Production Mode Validation (With `--production` flag)

### 18. Dependency Existence Check (`hdx_check_dependencies.ts`)

**Purpose**: Validates dependencies exist on cluster without deploying.

**Checks**:
- âœ… Lists functions/dictionaries in hdx_solutions (for shared resources)
- âœ… Lists functions/dictionaries in bundle's project (for bundle-specific resources)
- âœ… Checks each declared shared function exists (expected: `hdx_solutions_{name}`)
- âœ… Checks each declared shared dictionary exists (expected: `hdx_solutions_{name}`)
- âœ… Checks each declared bundle-specific function exists (expected: `{project}_{name}`)
- âœ… Checks each declared bundle-specific dictionary exists (expected: `{project}_{name}`)
- âœ… Verifies local files present
- âŒ **Fails if resource missing on cluster**
- âš ï¸ **Warns if local file missing**

**Success output**:
```
âœ“ All required dependencies exist on cluster
âœ“ All required local files present
```

**Failure output**:
```
âŒ Missing functions on cluster (hdx_solutions):
   - city_name (expected as: hdx_solutions_city_name)

âŒ Missing dictionaries on cluster (sample_project):
   - custom_dict (expected as: sample_project_custom_dict)

âš ï¸  Missing local definition files:
   - functions/city_name.json

ðŸ“‹ In production mode:
   â€¢ Resources must exist on cluster before deployment
   â€¢ Either create them manually or run without --production flag first
```

**Use case**: Validate bundle ready for production where resources already exist.

---

## Shared Resources Validation Details

### Resource Declaration Modes

**Explicit Mode** - Resources explicitly listed:
```json
{
  "shared_functions": ["city_name"],
  "required_functions": ["custom_parser"]
}
```
- All declared resources validated for local files
- No auto-discovery
- Resources created in correct projects

**Auto-Discovery Mode** - No resources declared:
```json
{
  "dependencies": {"hydrolix": {}}
}
```
- Scans filesystem for resources
- All discovered treated as bundle-specific
- No shared resources created

**Hybrid Mode** - Shared explicit, bundle auto-discover:
```json
{
  "shared_functions": ["city_name"]
  // Omit required_functions - auto-discovers bundle-specific
}
```
- Shared resources validated
- Bundle-specific auto-discovered
- Mixed creation in both projects

### Empty Array Behavior

**Important**: Empty arrays disable auto-discovery:

```json
{
  "required_functions": [],  // "Zero bundle-specific functions"
  "shared_functions": ["city_name"]
}
```

This configuration:
- âœ… Creates `city_name` in hdx_solutions
- âŒ Does NOT auto-discover bundle-specific functions
- To enable auto-discovery, **omit the field entirely**

### Cross-Project Resource References

Functions and dictionaries can reference resources from any project:

```sql
-- Bundle-specific calling shared (valid!)
CREATE FUNCTION sample_project_enrich AS
(ip) -> dictGet('hdx_solutions_geoip_city', 'city_name', ip)

-- Shared calling shared
CREATE FUNCTION hdx_solutions_country_name AS
(ip) -> dictGetString('hdx_solutions_geoip_locations', 'country', 
                      hdx_solutions_geoname_id(ip))
```

---

## Error Handling & Reporting

### Error Message Format

All errors include:
- Descriptive message
- File paths (when applicable)
- Expected vs actual values
- Actionable guidance

**Examples**:
```
âŒ Transform file is not valid JSON: 
   path=transformations/mcdn_akamai.json 
   error=Unexpected token at line 45

âŒ Shared function 'city_name' declared but file not found.
   Expected: my-bundles/mcdn_test/functions/city_name.json
   Actions:
     1. Add city_name.json to functions/ folder, OR
     2. Remove 'city_name' from shared_functions in bundle.json
```

### Exit Codes

- **0**: All validation and tests passed
- **1**: Failure (exits immediately on first error)

### Warning vs Error

**Errors** (exit code 1):
- Missing required files (declared resources without local files)
- Invalid JSON syntax
- Missing required fields
- Duplicate names
- Invalid checksums
- Browser test failures
- Missing dependencies in production mode

**Warnings** (continue execution):
- Resource used in SQL but not declared
- Resource declared but not used
- Data insertion failed (continues deployment)

---

## Test Execution Flow Summary

```
1. DISCOVERY
   â””â”€â”€ Find bundle.json files in my-bundles/

2. PARSING & VALIDATION
   â”œâ”€â”€ Parse JSON, validate structure
   â”œâ”€â”€ Validate fields, formats, enums
   â”œâ”€â”€ Check files exist
   â””â”€â”€ Validate content

3. CROSS-BUNDLE VALIDATION
   â””â”€â”€ Check for global duplicates

4. SHARED RESOURCES (--local only)
   â”œâ”€â”€ Check/create hdx_solutions project
   â”œâ”€â”€ Create/verify shared functions
   â””â”€â”€ Create/verify shared dictionaries

5. BUNDLE RESOURCES (--local only)
   â”œâ”€â”€ Extract zips
   â”œâ”€â”€ Auto-discover (if not declared)
   â”œâ”€â”€ Create bundle-specific functions
   â””â”€â”€ Create bundle-specific dictionaries

6. DATA PIPELINE (--local only)
   â”œâ”€â”€ Create tables
   â”œâ”€â”€ Deploy transforms
   â”œâ”€â”€ Insert sample data
   â””â”€â”€ Create summary tables

7. GRAFANA (--local or --local-dashboard-only)
   â”œâ”€â”€ Setup container
   â”œâ”€â”€ Create datasource
   â”œâ”€â”€ Deploy dashboards
   â””â”€â”€ Create alert rules

8. BROWSER TESTING
   â”œâ”€â”€ Launch Chrome
   â”œâ”€â”€ Load dashboard
   â”œâ”€â”€ Monitor errors
   â””â”€â”€ Report results
```

---

## Validation Module Summary

| Module | Purpose | Type |
|--------|---------|------|
| `valid_base_url` | Base URL format | Error |
| `naming_is_valid` | Naming conventions | Error |
| `no_duplicate_tokens` | Duplicate names (bundle) | Error |
| `no_bad_checksums` | SHA256 integrity | Error |
| `transforms_are_valid` | Transform structure | Error |
| `sample_data_exists` | Sample data presence | Error |
| `dashboard_is_valid` | Dashboard structure | Error |
| `alert_rules_are_valid` | Alert rules structure | Error |
| `summary_table` | Summary table refs | Error |
| `check_dependencies` | Resource files | Warning |
| `no_global_duplicates` | Duplicate names (global) | Error |

---

## Shared Resources Deployment Details

### Function Deployment Flow

```
1. Check if hdx_solutions_city_name exists
   â”œâ”€ YES â†’ Log "âœ“ exists", skip
   â””â”€ NO â†’ Continue

2. Read functions/city_name.json
   â””â”€ FAIL if missing

3. Replace __SHARED_PROJECT__ â†’ hdx_solutions

4. POST to hdx_solutions functions endpoint
   â””â”€ Creates as: hdx_solutions_city_name

5. Log "âœ“ Created"
```

### Dictionary Deployment Flow

```
1. Check if hdx_solutions_geoip_city exists
   â”œâ”€ YES â†’ Log "âœ“ exists", skip
   â””â”€ NO â†’ Continue

2. Find files:
   â”œâ”€ Search dictionaries/geoip_city.json + .csv/.yaml
   â””â”€ Then .extracted/geoip_city.json + .csv/.yaml
   â””â”€ FAIL if not found

3. Upload data file to hdx_solutions

4. POST definition to hdx_solutions dictionaries endpoint
   â””â”€ Creates as: hdx_solutions_geoip_city

5. Log "âœ“ Created"
```

### Idempotency

Shared resources are idempotent - running deployment multiple times:
- First run: Creates shared resources
- Subsequent runs: Reuses existing shared resources (logs "âœ“ exists")
- Different bundles: Share same resources (no duplication)

---

## CI/CD Integration

```yaml
# GitHub Actions Example
- name: Validate Bundle
  run: deno run --allow-all src/main.ts mcdn_test

# Exit code 0 = success, 1 = failure
```

**Benefits**: Fast validation (~30s), clear errors, no deployment needed, validates shared resource declarations

---

## Getting Help

For validation failures:
1. Read error message and file path
2. Review validation rules in this document
3. Check [BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md) for format
4. Examine example bundles in `my-bundles/`
5. Verify resources in correct project (hdx_solutions vs bundle project)
6. Check template variables use correct prefix (`__SHARED_PROJECT__` vs `__PROJECT_NAME__`)
### 17. Grafana Plugin Validation

**Plugin Detection** (`grafana/grafana_plugins_check.ts`):
- Queries all deployed dashboards via Grafana API (`/api/dashboards/uid/{uid}`)
- Extracts panel types from live dashboard JSON  
- Queries installed plugins (`/api/plugins`)
- Identifies external plugins vs built-in panels
- Compares required plugins against installed plugins
- Reports missing plugins with installation instructions
- **Default**: Warns but continues deployment
- **With `--strict-plugins`**: Fails deployment if plugins missing

**Runs**: Only with `--local` or `--local-dashboard-only` flags

**Success output**:
```
Plugin check: All dashboards use built-in panels only
```

**Warning output** (default mode):
```
WARNING: Missing plugins detected!
Missing: marcusolsson-treemap-panel
Used in: "CDN Global View" dashboard
Fix: Update grafana/container.ts
```

**Error output** (`--strict-plugins` mode):
```
ERROR: Missing required Grafana plugins!
Plugin validation failed
Exit code: 1
```

**Installed plugins output**:
```
Using 1 external plugin (installed):
Treemap Panel - 3 panels across 2 dashboards
```

---