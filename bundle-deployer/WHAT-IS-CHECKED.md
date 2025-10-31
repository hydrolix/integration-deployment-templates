# Validation & Testing Coverage

This document outlines the comprehensive validation and testing performed by the TypeScript/Deno Bundle Deployer. The system ensures that integration bundles meet all quality, consistency, and functional requirements before and during deployment.

## Overview

The Bundle Deployer performs validation and testing in multiple phases:

1. **Individual Bundle Validation** - Each bundle is validated independently
2. **Global Cross-Bundle Validation** - All bundles are validated together for conflicts
3. **Integration Testing** (Optional with `--local`) - Functional testing with live Hydrolix and Grafana
4. **Browser Testing** (Optional with `--local` or `--local-dashboard-only`) - Headless Chrome validation

## Validation Phase (Always Runs)

These checks run for every command, regardless of flags:

### 1. Base URL Validation (`valid_base_url.ts`)

**Purpose**: Ensures bundle references the correct GitHub repository location.

**Implementation**:
```typescript
export function run(base: string, bundle: Bundle): void {
  const checkBaseUrl = 
    `https://github.com/hydrolix/integration-deployment-templates/blob/main/${base}`;
  
  if (bundle.base_url !== checkBaseUrl) {
    throw new Error(`Invalid base_url should be: '${checkBaseUrl}'`);
  }
}
```

**Checks**:
- ✅ `bundle.base_url` matches expected GitHub URL format
- ✅ Path includes bundle directory name
- ✅ Points to main branch

**Expected Format**:
```
https://github.com/hydrolix/integration-deployment-templates/blob/main/my-bundles/[bundle_name]
```

**Failure Examples**:
```
âŒ Invalid my_bundle base_url should be this: 'https://...'
âŒ Base URL points to wrong repository
âŒ Base URL uses incorrect branch name
```

---

### 2. Naming Convention Validation (`naming_is_valid.ts`)

**Purpose**: Enforces consistent naming conventions across bundle components.

**Checks**:

#### Method-Title Consistency
- ✅ `firehose` → UI title contains "Amazon Data Firehose", "AWS Firehose", or "Kinesis Data Firehose"
- ✅ `s3` → UI title contains "Amazon S3" or "AWS S3"
- ✅ `kinesis` → UI title contains "Amazon Kinesis" or "AWS Kinesis"

#### Source-Title Consistency
- ✅ `waf` source → UI title contains "WAF" (case-insensitive)

#### Bundle Name Requirements
- ✅ Bundle name includes both source and method (case-insensitive)
- ✅ Only alphanumeric characters, underscores, and dashes

#### Version Format
- ✅ Semantic versioning format: `X.Y.Z` (e.g., `1.0.0`)
- ✅ Exactly two dots in version string

#### Maintainer Format
- ✅ Valid email address (contains `@` and `.`)

#### Description Requirements
- ✅ Non-empty, non-whitespace description

**Failure Examples**:
```
âŒ docs.method.full_title 'Simple S3' does not match method 'firehose'
âŒ Source title should contain 'WAF' when source is 'waf'
âŒ Name 'simple' must include 'cloudfront' and 'kinesis'
âŒ Version 1.0 should follow semantic versioning format (e.g., 1.0.0)
âŒ Maintainer should be a valid email address
```

---

### 3. Duplicate Token Validation (`no_duplicate_tokens.ts`)

**Purpose**: Prevents naming conflicts within a single bundle.

**Checks**:

#### Table Name Validation
- ✅ No duplicate table names within the bundle
- ✅ Table names ≥ 3 characters
- ✅ Table names start with a letter (a-z, A-Z)
- ✅ Table names contain only alphanumeric characters and underscores
- ✅ No empty or truncated table names

#### Dashboard Variable Validation
- ✅ No duplicate `dashboard_var` values within the bundle
- ✅ Each table has a unique dashboard variable identifier

**Failure Examples**:
```
âŒ Duplicate table name mcdn_test
âŒ Missing or truncated table name '{}'
âŒ Invalid table name '123table' - must start with a letter
âŒ Invalid table name 'table-name' - only letters, digits, and underscores allowed
âŒ Duplicate database_var __TABLE_NAME__
```

---

### 4. Checksum Validation (`no_bad_checksums.ts`)

**Purpose**: Ensures file integrity through SHA256 checksum verification.

**Checks**:
- ✅ Dashboard file checksums (if provided)
- ✅ Transform file checksums (if provided)
- ✅ Summary table SQL file checksums (if provided)
- ✅ Computed checksums match declared checksums
- ✅ File accessibility for checksum computation

**Implementation**:
```typescript
async function generateSha256(input: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}
```

**Failure Examples**:
```
âŒ SHA256 abc123...def456 does not match for local file transformations/mcdn_akamai.json
âŒ Cannot read file for checksum calculation
```

**Note**: Checksums are optional. If not provided, no validation is performed.

---

### 5. Transform File Validation (`transforms_are_valid.ts`)

**Purpose**: Validates all transformation files referenced in the bundle.

**Checks**:
- ✅ All transform files exist and are readable
- ✅ Transform files contain valid JSON syntax
- ✅ Required `name` field exists and is non-empty string
- ✅ No duplicate transform names within a table
- ✅ `subtype` field (if present) must be "firehose"

**Failure Examples**:
```
âŒ Transform file is not valid JSON: path=transformations/mcdn_akamai.json error=Unexpected token
âŒ Transform file missing required 'name' field: path=...
âŒ Transform file 'name' field is not a string: path=...
âŒ Transform file has empty 'name' field: path=...
âŒ Duplicated transform name 'mcdn_akamai' path=...
âŒ Transform file has invalid subtype 'custom', must be 'firehose': path=...
```

---

### 6. Sample Data Validation (`sample_data_exists.ts`)

**Purpose**: Ensures all transforms include sample data for testing.

**Checks**:
- ✅ Each transform contains `settings.sample_data`
- ✅ Sample data is either:
  - Non-empty JSON object
  - Non-empty string (after trimming)
- ✅ Sample data has content (not just `{}` or `""`)

**Failure Examples**:
```
âŒ No Sample data in transformation full_path=transformations/mcdn_akamai.json
âŒ Sample data is empty object: {}
âŒ Sample data is empty string
```

---

### 7. Dashboard Validation (`dashboard_is_valid.ts`)

**Purpose**: Validates Grafana dashboard files and their template variables.

**Checks**:

#### Required Template Variables
- ✅ `__DASHBOARD_UUID__` - Dashboard identifier placeholder
- ✅ `__DATASOURCE__` - Data source placeholder
- ✅ `__PROJECT_NAME__` - Project name placeholder
- ✅ All table `dashboard_var` values from bundle configuration
- ✅ All summary table `dashboard_var` values (if defined)

#### Dashboard Structure
- ✅ File exists and is readable
- ✅ Contains valid JSON
- ✅ Top-level `dashboard` object exists
- ✅ No hardcoded `id` field (must use `__DASHBOARD_UUID__`)

**Validation for Multiple Dashboards**:
- ✅ Primary dashboard (`bundle.dashboard`)
- ✅ Additional dashboards (`bundle.other_dashboards[]`)

**Failure Examples**:
```
âŒ Dashboard must have __DATASOURCE__ full_path=dashboards/CDN Dashboard.json
âŒ Invalid JSON full_path=dashboards/CDN Dashboard.json error=Unexpected token
âŒ Invalid dashboard - top element must be dashboard. full_path=...
âŒ Invalid dashboard - cannot have Id set. full_path=...
```

---

### 8. Alert Rules Validation (`alert_rules_are_valid.ts`)

**Purpose**: Validates alert rules file structure (if present).

**Checks**:
- ✅ Alert rules file exists and is readable (if defined)
- ✅ Contains valid JSON
- ✅ Required `apiVersion` field present
- ✅ `groups` is an array with at least one group
- ✅ Each group has required fields: `name`, `folder`, `interval`
- ✅ Each group has `rules` array with at least one rule
- ✅ Each rule has required fields: `uid`, `title`, `condition`, `data`
- ✅ Rule `data` is an array

**Failure Examples**:
```
âŒ alert_rules.apiVersion is required
âŒ alert_rules.groups must be an array
âŒ Group 0: name is required
âŒ Group alerts (0): rules must contain at least one rule
âŒ Group alerts, Rule 0: uid is required
```

**Note**: Alert rules are optional. If not defined in bundle, validation is skipped.

---

### 9. Summary Table Validation (`summary_table.ts`)

**Purpose**: Validates summary table references and prevents duplicates.

**Checks**:
- ✅ Summary table `parent_table_name` references a valid table in the bundle
- ✅ No duplicate `dashboard_var` values in summary tables
- ✅ Summary table names are unique

**Failure Examples**:
```
âŒ Duplicated-Summary-Dashboard-Var bundle=mcdn_test summary_table=mcdn_summary_min dashboard_var=__SUMMARY_TABLE_1__
âŒ Invalid-Parent-Table-Reference bundle=mcdn_test summary_table=mcdn_summary_min parent_table_name=nonexistent_table
```

**Note**: Summary tables are optional. If not defined, validation is skipped.

---

### 10. Dependency Validation (`check_dependencies.ts`)

**Purpose**: Validates function and dictionary dependencies.

**Checks**:

#### Function Files
- ✅ Local JSON file exists for each declared function (`functions/{name}.json`)
- ⚠️ Warns if function declared but file missing

#### Dictionary Files
- ✅ Local JSON definition exists for each declared dictionary (`dictionaries/{name}.json`)
- ✅ Data file exists for each dictionary (`.csv`, `.yaml`, `.yml`, or `.tsv`)
- ⚠️ Warns if dictionary declared but files missing

#### SQL Reference Validation
- ✅ Scans transform SQL for function calls
- ✅ Matches function calls to declared functions
- ✅ Scans for dictionary usage (`dictGet`, `dictGetString`, `dictGetOrDefault`)
- ⚠️ Warns if dictionary used but not declared
- ⚠️ Warns if declared but never used

**Warning Examples**:
```
âš ï¸  WARNING: Function 'city_name' declared but no file: functions/city_name.json
âš ï¸  WARNING: Dictionary 'ua_cat_dict' has definition but no data file
âš ï¸  WARNING: Transform uses dictionary 'geoip_dict' but it's not declared in dependencies
âš ï¸  INFO: Function 'unused_func' is declared but not used in any transforms
```

**Note**: This validation produces warnings only, does not fail the build.

---

## Global Cross-Bundle Validation

These checks run after all individual bundles are validated:

### 11. Global Duplicate Prevention (`no_global_duplicates.ts`)

**Purpose**: Prevents conflicts across all bundles in the repository.

**Checks**:
- ✅ No duplicate bundle names across all bundles
- ✅ No duplicate UI source titles across all bundles
- ✅ No duplicate table names across all bundles
- ✅ No duplicate base URLs across all bundles

**Implementation**:
```typescript
export function run(bundles: Bundle[]): void {
  const tokens = new Map<string, number>();
  
  for (const bundle of bundles) {
    tokens.set(bundle.name, (tokens.get(bundle.name) || 0) + 1);
    tokens.set(bundle.ui.source.full_title, (tokens.get(bundle.ui.source.full_title) || 0) + 1);
    // ... check tables, base_url
  }
  
  // Fail if any token appears more than once
}
```

**Failure Examples**:
```
âŒ Duplicated-Bundle-Name url=https://... error=mcdn_test
âŒ Duplicated-UI-Source-Name url=https://... error=MCDN TEST
âŒ Duplicated-Name count=2 table=mcdn_test
```

---

## Integration Testing Phase (With `--local` flag)

These tests run only when deploying locally:

### 12. Zip Extraction (`hdx.ts::ensureZipExtracted`)

**Purpose**: Automatically extract dictionary zip files.

**Checks**:
- ✅ Detects `dictionaries.zip` in bundle
- ✅ Extracts to `.extracted/` directory (gitignored)
- ✅ Flattens directory structure (uses `-j` flag)
- ✅ Skips if already extracted

**Implementation**:
```typescript
const process = new Deno.Command("unzip", {
  args: ["-j", "-q", "-o", zipPath, "-d", extractDir],
});
```

**Output**:
```
  Extracting dictionaries.zip...
  âœ" Extracted dictionaries.zip to .extracted/
```

---

### 13. Auto-Discovery (`hdx.ts::discoverFunctions`, `hdx.ts::discoverDictionaries`)

**Purpose**: Automatically discover functions and dictionaries from files.

**Function Discovery**:
- ✅ Scans `functions/` directory for `.json` files
- ✅ Returns list of function names (without `.json` extension)

**Dictionary Discovery**:
- ✅ Scans `.extracted/` directory (from zip) first
- ✅ Then scans `dictionaries/` directory (for overrides)
- ✅ Matches `.json` definition files with data files (`.csv`, `.yaml`, `.yml`, `.tsv`)
- ✅ Returns list of dictionary names with both files present
- ✅ Skips duplicates (prioritizes root folder over `.extracted/`)

**Output**:
```
  Scanning for functions in functions/...
    Found: city_name
    Found: breadcrumbs
  Scanning for dictionaries in dictionaries/.extracted...
    Found: ua_cat_dict (.json + .yaml)
    Found: geoip_city_blocks_ipv4 (.json + .csv)
```

---

### 14. Function Creation (`hdx.ts::checkAndCreateFunction`)

**Purpose**: Creates custom SQL functions in Hydrolix.

**Checks**:
- ✅ Function doesn't already exist on cluster
- ✅ Function JSON file exists locally
- ✅ Function JSON is valid
- ✅ Replaces `__PROJECT_NAME__` with actual project name
- ✅ Creates function via API (becomes `{project}_{name}`)

**Template Replacement**:
```typescript
// Input: functions/city_name.json
{
  "sql": "(ip) -> dictGetString('__PROJECT_NAME___geoip_dict', 'city', ip)"
}

// Deployed as: sample_project_city_name
{
  "sql": "(ip) -> dictGetString('sample_project_geoip_dict', 'city', ip)"
}
```

**Output**:
```
Checking function: city_name...
  Creating function city_name (will become sample_project_city_name)...
  âœ" Created function city_name
```

**Failure Handling**:
- ⚠️ Warns if function file missing (continues deployment)
- ⚠️ Warns if function creation fails (continues deployment)

---

### 15. Dictionary Creation (`hdx.ts::checkAndCreateDictionary`)

**Purpose**: Creates lookup dictionaries in Hydrolix.

**Checks**:
- ✅ Dictionary doesn't already exist on cluster
- ✅ Dictionary definition (`.json`) exists
- ✅ Dictionary data file (`.csv`, `.yaml`, `.yml`, `.tsv`) exists
- ✅ Uploads data file to Hydrolix
- ✅ Creates dictionary definition (becomes `{project}_{name}`)

**File Search Priority**:
1. `dictionaries/` (root - for custom overrides)
2. `dictionaries/.extracted/` (from zip)

**Upload Process**:
```typescript
// 1. Upload data file (stripped extension: ua_cat_dict.yaml → ua_cat_dict)
const formData = new FormData();
formData.append('file', new Blob([fileContent]), fileName);
formData.append('name', baseFileName);  // No extension

// 2. Create definition (references uploaded file by base name)
{
  "name": "ua_cat_dict",
  "settings": {
    "filename": "ua_cat_dict",  // No extension
    // ...
  }
}
```

**Output**:
```
Checking dictionary: ua_cat_dict...
  Found files: dictionaries/ua_cat_dict.json + dictionaries/ua_cat_dict.yaml
  Uploading dictionary file: ua_cat_dict.yaml (as ua_cat_dict)...
  âœ" Uploaded dictionary file: ua_cat_dict
  Creating dictionary definition: ua_cat_dict (will become sample_project_ua_cat_dict)...
  âœ" Created dictionary definition
  âœ" Created dictionary ua_cat_dict
```

**Failure Handling**:
- ⚠️ Warns if dictionary files missing (continues deployment)
- ⚠️ Warns if dictionary creation fails (continues deployment)

---

### 16. Table Creation (`hdx.ts::createTable`)

**Purpose**: Creates data tables in Hydrolix.

**Checks**:
- ✅ Creates table with proper settings
- ✅ Returns table UUID for transform attachment
- ✅ Waits 30 seconds for table initialization

**Settings**:
```typescript
{
  name: tableName,
  description: "testing",
  settings: {
    age: {
      max_age_days: 1,  // Auto-delete after 1 day
    },
    merge: {
      enabled: false,  // No merging for test tables
    },
  },
}
```

**Output**:
```
Creating table: mcdn_test
Waiting for table to be ready...
```

---

### 17. Transform Deployment (`hdx.ts::addTransformToTable`)

**Purpose**: Attaches data transformation schemas to tables.

**Checks**:
- ✅ Transform JSON is valid
- ✅ Replaces `__PROJECT_NAME__` in SQL transforms
- ✅ Adds transform to table via API
- ✅ Retries up to 5 times on failure (with exponential backoff)

**Template Replacement**:
```typescript
// Input SQL:
SELECT __PROJECT_NAME___city_name(ip) AS city
FROM {STREAM}

// Deployed SQL:
SELECT sample_project_city_name(ip) AS city
FROM {STREAM}
```

**Retry Logic**:
- Attempts: 5 max
- Base delay: 1 second
- Max delay: 30 seconds
- Exponential backoff: `delay = baseDelay * 2^attempt`

**Output**:
```
  âœ" Transform validation successful
  âœ" Transform deployed: mcdn_akamai_ds2
```

**Failure Examples**:
```
âŒ Transform validation failed (attempt 1/5):
   Status: 400
   Error: Unknown function sample_project_city_name
```

---

### 18. Data Insertion (`hdx.ts::insertIntoTable`)

**Purpose**: Inserts sample data into tables for testing.

**Checks**:
- ✅ Sample data exists in transform settings
- ✅ Converts single objects to arrays
- ✅ Sends data to Hydrolix ingest endpoint
- ✅ Retries up to 20 times on failure (with exponential backoff)
- ✅ Waits 30 seconds for table to be ready before inserting

**Retry Logic**:
- Attempts: 20 max
- Base delay: 1 second
- Max delay: 60 seconds
- Retryable errors: 5xx, 408, 429
- Non-retryable errors: 4xx (except 408, 429)

**Output**:
```
Found sample data for transform mcdn_akamai_ds2, preparing to insert...
Waiting for table to be ready for data...
Inserting sample data into sample_project.mcdn_test with transform mcdn_akamai_ds2...
âœ" Successfully inserted sample data into sample_project.mcdn_test
```

**Failure Handling**:
- ⚠️ Warns if insertion fails (continues deployment)
- Deployment completes but dashboard may show "No data"

---

### 19. Summary Table Creation (`hdx.ts::createSummaryTable`)

**Purpose**: Creates pre-aggregated summary views.

**Checks**:
- ✅ Summary SQL file exists
- ✅ SQL file is readable
- ✅ Replaces `__PROJECT_NAME__` in SQL
- ✅ Replaces `__TABLE_NAME__` with parent table name
- ✅ Creates summary table via API

**Template Replacement**:
```sql
-- Input SQL:
SELECT 
  toStartOfMinute(timestamp) AS minute,
  COUNT(*) AS requests
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY minute

-- Deployed SQL:
SELECT 
  toStartOfMinute(timestamp) AS minute,
  COUNT(*) AS requests
FROM sample_project.mcdn_test
GROUP BY minute
```

**Output**:
```
âœ" Created summary table: mcdn_summary_min
```

---

### 20. Grafana Datasource Creation (`grafana/interface.ts::createDatalink`)

**Purpose**: Creates Hydrolix datasource in Grafana.

**Checks**:
- ✅ Creates datasource with proper configuration
- ✅ Returns datasource UID for dashboard reference
- ✅ Waits 2 seconds for Grafana to settle

**Configuration**:
```typescript
{
  name: "Bundle Testing",
  type: "hydrolix-hydrolix-datasource",
  access: "proxy",
  jsonData: {
    default_database: projectName,
    host: BUNDLE_TESTING_CLUSTER,
    port: 9440,
    protocol: "native",
    query_timeout: "600",
    secure: true,
    username: BUNDLE_TESTING_USERNAME,
  },
  secureJsonData: {
    password: BUNDLE_TESTING_PASSWORD,
  },
  readOnly: true,
}
```

**Output**:
```
Creating Grafana datasource for project sample_project...
âœ" Created Grafana datasource with UID: abc123def456
```

---

### 21. Dashboard Deployment (`grafana/interface.ts::createDashboard`)

**Purpose**: Deploys Grafana dashboards with template variable replacement.

**Checks**:
- ✅ Dashboard JSON is valid
- ✅ Replaces all template variables
- ✅ Imports dashboard to Grafana
- ✅ Returns dashboard UID

**Template Replacements**:
```typescript
// Input:
__PROJECT_NAME__ → "sample_project"
__DATASOURCE__ → "abc123def456"
__DASHBOARD_UUID__ → crypto.randomUUID()
__TABLE_NAME__ → "mcdn_test"
__SUMMARY_TABLE_NAME_1__ → "mcdn_summary_min"
```

**Multiple Dashboard Support**:
- Primary dashboard: `bundle.dashboard`
- Additional dashboards: `bundle.other_dashboards[]`

**Output**:
```
âœ" Created primary dashboard (UID: xyz789)
Creating additional dashboard: dashboards/Raw Logs.json
âœ" Created dashboard: dashboards/Raw Logs.json
```

---

### 22. Alert Rules Deployment (`grafana/interface.ts::createAlertRules`)

**Purpose**: Deploys alert rules alongside dashboards.

**Checks**:
- ✅ Alert rules JSON is valid
- ✅ Creates folders as needed
- ✅ Replaces template variables
- ✅ Creates each rule individually via API
- ✅ Cleans UI-only fields before submission

**Template Replacements**:
```typescript
__PROJECT_NAME__ → "sample_project"
__DATASOURCE__ → "abc123def456"
__DASHBOARD_UUID__ → dashboard UID
__TABLE_NAME__ → "mcdn_test"
```

**Folder Management**:
```typescript
// 1. Check if folder exists
const folders = await fetch('/api/folders');

// 2. Create if missing
if (!existingFolder) {
  await fetch('/api/folders', {
    method: 'POST',
    body: JSON.stringify({ title: folderTitle }),
  });
}
```

**Rule Cleaning**:
```typescript
// Remove UI-only fields before API submission
const { notification_settings, isPaused, templating, ...cleanRule } = rule;
```

**Output**:
```
Creating 2 alert rule group(s)...
  Creating folder "CDN Alerts"...
  âœ" Created folder "CDN Alerts" (uid: folder123)
  Creating rule group "Traffic Monitoring" with 3 rule(s)...
    Creating rule "High Error Rate"...
    âœ" Created rule "High Error Rate" (id: 1)
    Creating rule "Low Traffic"...
    âœ" Created rule "Low Traffic" (id: 2)
  âœ" Created rule group "Traffic Monitoring"
âœ" Successfully created all alert rules
```

**Failure Examples**:
```
âŒ Failed to create alert rule "High Error Rate": Invalid query expression
âŒ Failed to create folder "CDN Alerts": Folder already exists
```

---

## Browser Testing Phase (With `--local` or `--local-dashboard-only`)

These tests run only when testing locally with browser validation:

### 23. Headless Browser Testing (`headless_browser.ts`)

**Purpose**: Validates dashboards load correctly in Grafana using headless Chrome.

**Checks**:
- ✅ Grafana session authentication works
- ✅ Dashboard loads without errors
- ✅ No datasource errors (`Datasource \w+ was not found`)
- ✅ No 400 query errors in console
- ✅ 30-second wait for all panels to render

**Implementation**:
```typescript
// 1. Get Grafana session cookie
const { cookieName, cookieValue } = await getGrafanaSessionCookie();

// 2. Launch headless Chrome
const browser = await puppeteer.launch({
  headless: "new",
  args: ['--no-sandbox', '--disable-setuid-sandbox'],
});

// 3. Set viewport for dashboard rendering
await page.setViewport({ width: 1920, height: 4080 });

// 4. Monitor console for errors
page.on('console', (msg) => {
  const text = msg.text();
  if (badDatasourceRegex.test(text) || fourHundredRegex.test(text)) {
    datasourceErrorCount++;
  }
});

// 5. Load dashboard
await page.goto(dashboardUrl, {
  waitUntil: 'networkidle2',
  timeout: 120000,
});

// 6. Wait for all panels to load
await new Promise(resolve => setTimeout(resolve, 30000));
```

**Error Detection**:
- Datasource errors: `/Datasource \w+ was not found/`
- Query errors: `/400 \w+/`

**Output**:
```
Starting headless browser test for dashboard: xyz789
Got Grafana session cookie: grafana_session
Dashboard ID=xyz789
Navigating to: http://localhost:3000/d/xyz789/cdn-dashboard
Page loaded - Title: "CDN Dashboard", URL: http://localhost:3000/...
Page navigation completed
Waiting 30 seconds for all panels to load and query data...
Datasource errors: 0
Success! No datasource errors detected.
```

**Failure Examples**:
```
ERROR: Datasource not found - Datasource Hydrolix was not found
Datasource errors: 2
ERROR: Dashboard Errors=2
```

**Success Criteria**:
- ✅ `datasourceErrorCount === 0`
- ✅ Dashboard loads within 120 seconds
- ✅ No console errors during 30-second wait

---

## Production Mode Validation (With `--production` flag)

### 24. Dependency Existence Check (`hdx_check_dependencies.ts`)

**Purpose**: Validates dependencies exist on cluster before deployment.

**Checks**:

#### Functions
- ✅ Lists all functions on cluster
- ✅ Checks each declared function exists (with project prefix: `{project}_{name}`)
- ✅ Checks local function file exists
- ❌ Fails if function missing on cluster
- ⚠️ Warns if local file missing

#### Dictionaries
- ✅ Lists all dictionaries on cluster
- ✅ Checks each declared dictionary exists (with project prefix)
- ✅ Checks local definition file exists (`.json`)
- ✅ Checks local data file exists (`.csv`, `.yaml`, `.yml`, `.tsv`)
- ❌ Fails if dictionary missing on cluster
- ⚠️ Warns if local files missing

**Output - Success**:
```
âœ" All required dependencies exist on cluster
âœ" All required local files present
```

**Output - Failure**:
```
âŒ Missing functions on cluster:
   - city_name (expected as: sample_project_city_name)

âŒ Missing dictionaries on cluster:
   - ua_cat_dict (expected as: sample_project_ua_cat_dict)

âš ï¸  Missing local definition files:
   - functions/city_name.json
   - dictionaries/ua_cat_dict.json

ðŸ"‹ In production mode:
   â€¢ Resources must exist on cluster before deployment
   â€¢ Either create them manually or run without --production flag first
   â€¢ Local files should be included for documentation and validation
```

**Use Case**: Validate bundle is ready for production deployment where resources already exist.

---

## Error Handling & Reporting

### Error Message Format

All validation errors include:
- ✅ Descriptive error message
- ✅ File paths (when applicable)
- ✅ Expected vs actual values
- ✅ Actionable guidance for resolution

**Examples**:
```
âŒ Transform file is not valid JSON: 
   path=transformations/mcdn_akamai.json 
   error=Unexpected token at line 45
   
âŒ SHA256 abc123...def does not match for local file 
   path=transformations/mcdn_akamai.json
   expected=abc123...
   actual=def456...
   
âŒ Dashboard must have __DATASOURCE__ 
   full_path=dashboards/CDN Dashboard.json
```

---

### Exit Codes

- **0**: All validation and tests passed successfully
- **1**: Validation failure, deployment failure, or browser test failure

The system exits immediately on the first failure encountered.

---

### Warning vs Error

**Errors** (exit code 1):
- ❌ Missing required files
- ❌ Invalid JSON syntax
- ❌ Missing required fields
- ❌ Duplicate names
- ❌ Invalid checksums
- ❌ Browser test failures

**Warnings** (continue execution):
- ⚠️ Missing function file (dependency declared but not in bundle)
- ⚠️ Missing dictionary file (dependency declared but not in bundle)
- ⚠️ Function/dictionary used but not declared
- ⚠️ Function/dictionary declared but not used
- ⚠️ Function creation failed (continues deployment)
- ⚠️ Dictionary creation failed (continues deployment)
- ⚠️ Data insertion failed (continues deployment)

---

## Test Execution Flow Summary

```
1. DISCOVERY
   └── Find all bundle.json files in my-bundles/

2. PARSING & STRUCTURE VALIDATION
   ├── Parse bundle.json
   ├── Validate required fields
   ├── Validate field formats
   └── Validate enum values

3. FILE VALIDATION
   ├── Check all referenced files exist
   ├── Validate JSON syntax
   ├── Verify checksums (if provided)
   └── Check sample data presence

4. CONTENT VALIDATION
   ├── Validate dashboard structure
   ├── Check template variables
   ├── Validate transforms
   ├── Check function/dictionary references
   ├── Validate alert rules (if present)
   └── Validate summary tables (if present)

5. CROSS-BUNDLE VALIDATION
   ├── Check for duplicate bundle names
   ├── Check for duplicate UI titles
   └── Check for duplicate table names

6. INTEGRATION TESTING (--local only)
   ├── Extract dictionary zips
   ├── Auto-discover resources
   ├── Create functions
   ├── Create dictionaries
   ├── Create tables
   ├── Add transforms
   ├── Insert sample data
   └── Create summary tables

7. GRAFANA DEPLOYMENT (--local or --local-dashboard-only)
   ├── Kill old Grafana container
   ├── Start new Grafana container
   ├── Create datasource
   ├── Deploy dashboards
   └── Create alert rules

8. BROWSER TESTING (--local or --local-dashboard-only)
   ├── Launch headless Chrome
   ├── Navigate to dashboard
   ├── Monitor for errors
   └── Report results

9. CLEANUP
   └── Close browser, report success/failure
```

---

## Validation Module Summary

| Module | Purpose | Failures | Warnings |
|--------|---------|----------|----------|
| `valid_base_url` | Base URL format | ❌ | - |
| `naming_is_valid` | Naming conventions | ❌ | - |
| `no_duplicate_tokens` | Duplicate names (bundle) | ❌ | - |
| `no_bad_checksums` | SHA256 integrity | ❌ | - |
| `transforms_are_valid` | Transform structure | ❌ | - |
| `sample_data_exists` | Sample data presence | ❌ | - |
| `dashboard_is_valid` | Dashboard structure | ❌ | - |
| `alert_rules_are_valid` | Alert rules structure | ❌ | - |
| `summary_table` | Summary table refs | ❌ | - |
| `check_dependencies` | Function/dict files | - | ⚠️ |
| `no_global_duplicates` | Duplicate names (global) | ❌ | - |

---

## CI/CD Integration

The validation system is designed for CI/CD pipelines:

```yaml
# GitHub Actions Example
- name: Validate Bundle
  run: deno run --allow-all src/main.ts mcdn_test
  
# Exit code 0 = success
# Exit code 1 = failure (prevents merge)
```

**Benefits**:
- Fast validation (~30 seconds)
- Clear error messages
- No deployment required
- Prevents bad bundles from merging

---

## Getting Help

For validation failures:
1. Read the error message carefully
2. Check the referenced file and line number
3. Review validation rules in this document
4. Consult example bundles in `my-bundles/`
5. Use cleanup tool for fresh starts
6. Review [BUNDLE-DETAILS.md](./BUNDLE-DETAILS.md) for format specification