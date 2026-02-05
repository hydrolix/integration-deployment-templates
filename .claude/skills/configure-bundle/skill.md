---
name: configure-bundle
description: Configure a Hydrolix integration deployment bundle with proper template variables, dashboard structure, and bundle.json. Use when the user wants to set up or fix a Hydrolix integration bundle.
user-invocable: true
---

# Configure Hydrolix Integration Bundle

You are helping configure a Hydrolix integration deployment bundle. Follow this process systematically:

## Phase 1: Discovery and Assessment

1. **Identify the bundle directory** (ask if not provided)
   - Look for structure: dashboards/, summaries/, transformations/, functions/
   - List all files found
   - Determine bundle location: `aws/` or `trafficpeak/`

2. **Check what exists:**
   - ✓ bundle.json (if missing, needs to be created)
   - ✓ Dashboard JSON files in dashboards/
   - ✓ Summary SQL files in summaries/ (optional)
   - ✓ Transformation and sample data in transformations/ (or transforms/)
   - ✓ Function definitions in functions/ (optional)

3. **Identify the source/vendor:**
   - Check directory name or existing files for clues
   - Ask user for: source name, bundle name, table name, maintainer email
   - Table name is typically "logs", "events", "siem", etc.

## Phase 2: Transform Organization and Cleanup

This phase normalizes and organizes all transformation files.

### 2a. Normalize Folder Name

**Check transformation folder name:**
- Look for folders: `transforms/` or `transformations/`
- Identify by checking for JSON files with transform structure:
  - Contains `"settings"` object
  - Contains `"output_columns"` array
  - Contains `"name"` field

**Action:**
- If folder is named `transforms/` (singular) → rename to `transformations/`
- Update any bundle.json references to the folder

### 2b. Organize Transform Files

**Detect transform file structure:**

Count transform files in the transformations/ folder:

**Case A: Optimal (Single Transform)**
- Has: `transform.json` + `sample_data.json`
- Action: ✅ Continue to cleanup steps

**Case B: Single Transform Needs Renaming**
- Has: One transform file with different name (e.g., `akamai.json`)
- Action: Rename to `transform.json`

**Case C: Multiple Transforms (Multi-Provider)**
- Has: Multiple transform files (e.g., `akamai (4).json`, `cloudflare (4).json`, `fastly (2).json`)
- Action: Create subdirectory structure

**For Case C, create subdirectories:**

1. Extract provider name from each filename:
   - `akamai (4).json` → provider: `akamai`
   - `cloudfront_firehose.json` → provider: `cloudfront_firehose`
   - Strip numbers, parentheses, special chars from filename

2. Create subdirectory structure:
   ```
   transformations/
   ├── akamai/
   ├── cloudflare/
   ├── cloudfront_firehose/
   └── fastly/
   ```

3. Move each transform to its subdirectory:
   - `akamai (4).json` → `transformations/akamai/akamai (4).json`

4. **For EACH subdirectory**, apply Case A/B logic:
   - If transform is named `transform.json` → ✅ done
   - If transform has different name → rename to `transform.json`

### 2c. Clean Transform Metadata Fields

**For EACH transform file** (whether in root transformations/ or subdirectories):

Remove these metadata fields if present:
- `"uuid"` - Internal system ID
- `"created"` - Creation timestamp
- `"modified"` - Modification timestamp
- `"url"` - API endpoint URL
- `"table"` - Table UUID reference

**Keep these fields:**
- `"type"` - Usually "json", keep it
- `"name"` - Transform name
- `"description"` - Transform description
- `"settings"` - All transform settings
- `"sample_data"` - Sample data (will process in next step)

### 2d. Extract and Validate Sample Data

**For EACH transform file:**

1. **Check for sample_data field in transform:**
   - If `sample_data` field is missing or empty → **BLOCKER**
   - Stop and display error:
     ```
     ❌ ERROR: Cannot continue configuration

     The transform file '{filename}' is missing sample_data.

     Action Required:
     - Contact the team that provided this bundle
     - Request they add sample_data to the transform file
     - Sample data must include at least one complete example record

     Cannot proceed with bundle configuration until sample_data is present.
     ```

2. **Extract sample_data from transform:**
   - Read the `sample_data` field value

3. **Validate and normalize format:**
   - If sample_data is an array: `[{...}, {...}]`
     - Take ONLY the first element: `[0]`
     - Result should be single object: `{...}`
   - If sample_data is already an object: `{...}`
     - Use as-is ✅
   - **REQUIRED:** Final format must be single object, NOT array

4. **Create sample_data.json file:**
   - Write the normalized sample data object to `sample_data.json`
   - Location:
     - Single transform: `transformations/sample_data.json`
     - Multiple transforms: `transformations/{provider}/sample_data.json`

5. **Update transform file:**
   - Replace the `sample_data` field in transform.json with the normalized object
   - Both files must match exactly

**Result:** Both `sample_data.json` and transform's `sample_data` field contain identical single object.

### 2e. Analyze SQL Transform and Fix Prefixes

**For EACH transform file:**

1. **Determine correct prefix based on bundle location:**
   - If bundle path contains `aws/` → correct prefix = `commons`
   - If bundle path contains `trafficpeak/` → correct prefix = `akamai`

2. **Parse sql_transform field to find:**

   **Function calls pattern:** `(reference|commons|akamai|[a-z_]+)_([a-z_]+)\(`
   - Examples: `reference_breadcrumbs(`, `akamai_city_name(`, `commons_edge_worker(`

   **Dictionary calls pattern:** `dictGet\('(reference|commons|akamai|[a-z_]+)_([a-z_]+)'`
   - Examples: `dictGet('reference_ua_cat_dict'`, `dictGet('commons_geoip_asn_blocks_ipv4'`

3. **Extract base names** (without prefix):
   - `reference_breadcrumbs` → base: `breadcrumbs`
   - `commons_ua_cat_dict` → base: `ua_cat_dict`
   - `akamai_city_name` → base: `city_name`

4. **Replace prefixes in sql_transform:**
   - Replace ALL instances of `(reference|commons|akamai)_` with `{correct_prefix}_`
   - Examples for trafficpeak bundle:
     - `reference_breadcrumbs(` → `akamai_breadcrumbs(`
     - `dictGet('reference_ua_cat_dict'` → `dictGet('akamai_ua_cat_dict'`
     - `commons_city_name(` → `akamai_city_name(`

5. **Collect unique base names for bundle.json:**
   - Create lists of unique function and dictionary base names (without prefixes)
   - These will populate bundle.json dependencies later

**Example transformation for trafficpeak bundle:**

Before:
```sql
reference_breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'k=([^,\\]]+)')
dictGet('reference_ua_cat_dict', 'ua_category', assumeNotNull(user_agent))
```

After:
```sql
akamai_breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'k=([^,\\]]+)')
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent))
```

## Phase 3: Create/Update bundle.json

Now that transforms are analyzed and cleaned, create or update bundle.json with complete information.

### 3a. Determine Bundle Method

**Count transforms:**
- If single transform → method depends on transform type
- If multiple transforms → method = `multi_stream`

**For single transform, detect method from directory/filename:**
- If name contains "firehose" → method = `firehose`
- If name contains "kinesis" → method = `kinesis`
- Otherwise → method = `http_streaming`

**For multiple transforms:**
- Bundle-level method = `multi_stream`
- Per-transform method:
  - If subdirectory name contains "firehose" → transform method = `firehose`
  - If subdirectory name contains "kinesis" → transform method = `kinesis`
  - Otherwise → transform method = `http_streaming`

### 3b. Build Transform References

**Single transform:**
```json
"tables": [
  {
    "dashboard_var": "__TABLE_NAME__",
    "name": "{table_name}",
    "transforms": [
      {
        "method": "http_streaming",
        "path": "transformations/transform.json",
        "sample": "transformations/sample_data.json"
      }
    ]
  }
]
```

**Multiple transforms:**
```json
"tables": [
  {
    "dashboard_var": "__TABLE_NAME__",
    "name": "{table_name}",
    "transforms": [
      {
        "method": "http_streaming",
        "path": "transformations/akamai/transform.json",
        "sample": "transformations/akamai/sample_data.json"
      },
      {
        "method": "http_streaming",
        "path": "transformations/cloudflare/transform.json",
        "sample": "transformations/cloudflare/sample_data.json"
      },
      {
        "method": "firehose",
        "path": "transformations/cloudfront_firehose/transform.json",
        "sample": "transformations/cloudfront_firehose/sample_data.json"
      }
    ]
  }
]
```

### 3c. Populate Dependencies

Using the base names collected from Phase 2e:

```json
"dependencies": {
  "hydrolix": {
    "required_dictionaries": [],
    "required_functions": [],
    "shared_dictionaries": ["ua_cat_dict", "geoip_asn_blocks_ipv4"],
    "shared_functions": ["breadcrumbs", "city_name", "edge_worker"]
  }
}
```

**Note:** List base names only (without prefixes). The actual prefixes are in the sql_transform.

### 3d. Complete bundle.json Structure

If bundle.json doesn't exist, create with this structure:

```json
{
  "base_url": "https://github.com/hydrolix/integration-deployment-templates/blob/main/{source}/{bundle_name}",
  "beta": true,
  "dashboard": {
    "path": "dashboards/{PRIMARY_DASHBOARD_FILE}.json",
    "project_var": "__PROJECT_NAME__"
  },
  "dependencies": {
    "hydrolix": {
      "required_dictionaries": [],
      "required_functions": [],
      "shared_dictionaries": [/* from Phase 2e */],
      "shared_functions": [/* from Phase 2e */]
    }
  },
  "metadata": {
    "channel_type": "AWS",
    "description": "{Bundle Description}",
    "maintainer": "{email}",
    "version": "1.0.0"
  },
  "method": "http_streaming", /* or multi_stream */
  "name": "{source}_{bundle_name}",
  "other_dashboards": [],
  "solution": true,
  "source": "{source}",
  "summary_tables": [],
  "tables": [/* from Phase 3b */],
  "ui": {
    "data_category": "security",
    "method": {
      "full_title": "Http Streaming",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/http.png"
    },
    "primary_url": "https://docs.hydrolix.io/docs/{source}-integration",
    "source": {
      "full_title": "{Unique Source Title}",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/{source}.png"
    }
  }
}
```

**Important bundle.json rules:**
- Primary dashboard goes in `dashboard.path`
- Additional dashboards go in `other_dashboards[]` array
- Each summary table needs `dashboard_var`, `name`, `parent_table_name`, and `sql.path`
- `ui.source.full_title` must be unique across all bundles
- Shared functions/dictionaries listed WITHOUT prefixes
- `tables[].name` should be set to the table name provided by the user

## Phase 4: Fix Summary SQL Files

For each `.sql` file in `summaries/`:

1. **Check for hardcoded table references:**
   - Search for patterns like `{vendor}.{table}` or `FROM {table}`

2. **Replace with template variables:**
   - Replace hardcoded table references → `__PROJECT_NAME__.__TABLE_NAME__`
   - Example: `akamai.siem` → `__PROJECT_NAME__.__TABLE_NAME__`

3. **Add to bundle.json:**
   ```json
   "summary_tables": [
     {
       "dashboard_var": "__SUMMARY_TABLE_NAME_1__",
       "name": "{summary_table_name}",
       "parent_table_name": "{parent_table_name}",
       "sql": {
         "path": "summaries/{filename}.sql"
       }
     }
   ]
   ```

## Phase 5: Fix Dashboard Structure

For EACH dashboard JSON file:

### 5a. Check Dashboard Wrapper
**Required structure:**
```json
{
  "dashboard": {
    "__elements": { ... },
    "__requires": [ ... ],
    ...all dashboard content...
  }
}
```

If missing the top-level `"dashboard"` wrapper, add it.

### 5b. Update Dashboard UID
Find the UID at the bottom of the dashboard:
- Replace hardcoded UID → `"uid": "__DASHBOARD_UUID__"`

### 5c. Fix Template Variables

**Check for old-style variables to replace:**
- `${VAR_TIMESTAMP}` → `timestamp` (literal column name)
- `${VAR_SIEM}` → `__PROJECT_NAME__.__TABLE_NAME__`
- Any other `${VAR_*}` patterns

**Configure template variables in the dashboard:**

**For ALL dashboards (primary and other):**
```json
{
  "name": "{table_var_name}",
  "type": "constant",
  "query": "__PROJECT_NAME__.__TABLE_NAME__",
  "current": {
    "text": "__PROJECT_NAME__.__TABLE_NAME__",
    "value": "__PROJECT_NAME__.__TABLE_NAME__"
  }
}
```

**Summary table variables - CRITICAL DISTINCTION:**

**PRIMARY dashboard ONLY:**
```json
{
  "name": "{summary_var_name}",
  "type": "constant",
  "query": "__SUMMARY_TABLE_NAME_1__",
  "current": {
    "text": "__SUMMARY_TABLE_NAME_1__",
    "value": "__SUMMARY_TABLE_NAME_1__"
  }
}
```

**OTHER dashboards ONLY:**
```json
{
  "name": "{summary_var_name}",
  "type": "constant",
  "query": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__",
  "current": {
    "text": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__",
    "value": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"
  }
}
```

**WHY THIS DIFFERENCE EXISTS:**
The Hydrolix validator code processes dashboards differently:
- **Primary dashboard:** Variables replaced in `deploy/default.rs` where `__SUMMARY_TABLE_NAME_X__` becomes full path `project.table`
- **Other dashboards:** Variables replaced in `grafana/dashboard.rs` where `__SUMMARY_TABLE_NAME_X__` becomes just `table_name`, so you need the `__PROJECT_NAME__.` prefix

### 5d. Choose Primary Dashboard

1. **Identify the primary dashboard** (usually the main overview/analysis dashboard)
2. **Add to bundle.json:**
   ```json
   "dashboard": {
     "path": "dashboards/{primary_dashboard}.json",
     "project_var": "__PROJECT_NAME__"
   }
   ```

3. **Add remaining dashboards:**
   ```json
   "other_dashboards": [
     {
       "path": "dashboards/{dashboard2}.json",
       "project_var": "__PROJECT_NAME__"
     },
     {
       "path": "dashboards/{dashboard3}.json",
       "project_var": "__PROJECT_NAME__"
     }
   ]
   ```

## Phase 6: Validation Summary

After making all changes, provide a summary:

```
✅ Transform Organization Complete:
   - Folder normalized to: transformations/
   - Transform count: {N} ({single/multiple})
   - Structure: {single transform.json OR subdirectories per provider}
   - Metadata fields cleaned: uuid, created, modified, url, table
   - Sample data validated: single object format
   - SQL prefixes replaced: {old_prefix}_ → {correct_prefix}_

✅ Created/Updated Files:
   - bundle.json (with correct method and dependencies)
   - transformations/{structure}
   - summaries/{files}.sql (template variables)
   - dashboards/{primary}.json (primary)
   - dashboards/{other}.json (other dashboards)

✅ Dependencies Populated:
   - shared_functions: [{list}]
   - shared_dictionaries: [{list}]
   - All prefixes corrected for {aws/trafficpeak} bundle

✅ Template Variables Configured:
   - __PROJECT_NAME__ → project name
   - __TABLE_NAME__ → base table name
   - __SUMMARY_TABLE_NAME_X__ → summary tables
   - __DATASOURCE__ → datasource UID
   - __DASHBOARD_UUID__ → generated UUID

✅ Key Patterns Applied:
   - Transform methods: {http_streaming/firehose/multi_stream}
   - SQL prefixes: {commons/akamai}_ for functions/dictionaries
   - Primary dashboard: __SUMMARY_TABLE_NAME_X__ (no prefix)
   - Other dashboards: __PROJECT_NAME__.__SUMMARY_TABLE_NAME_X__ (with prefix)
   - Regular tables: __PROJECT_NAME__.__TABLE_NAME__ (all dashboards)

⚠️ Important Notes:
   - ui.source.full_title must be unique across all bundles
   - Sample data is single object (not array)
   - All function/dictionary prefixes match bundle location
   - Test deployment to verify all queries work correctly
```

## Reference: Variable Substitution Patterns

| Variable | Used In | Replacement | Example |
|----------|---------|-------------|---------|
| `__PROJECT_NAME__` | All | Project name | `bundle_verification` |
| `__TABLE_NAME__` | All | Table name only | `logs` |
| `__SUMMARY_TABLE_NAME_X__` | Primary dash | Full path | `project.summary_table` |
| `__SUMMARY_TABLE_NAME_X__` | Other dash | Table name only | `summary_table` |
| `__DATASOURCE__` | All | Datasource UID | Generated |
| `__DASHBOARD_UUID__` | All | Dashboard UID | Generated |

## Common Issues to Check

1. **Array-wrapped sample data:** If `sample_data.json` starts with `[`, remove the array wrapper
2. **Wrong SQL prefixes:** aws bundles must use `commons_`, trafficpeak must use `akamai_`
3. **Missing sample data:** Transform files must have sample_data field
4. **Duplication:** If seeing `project.project.table`, primary dashboard has wrong pattern
5. **Missing project:** If seeing just `table_name`, other dashboard missing `__PROJECT_NAME__.` prefix
6. **Syntax errors in queries:** Old `${VAR_*}` variables not replaced
7. **Missing wrapper:** Dashboard content not wrapped in `"dashboard": { }`
8. **Wrong transform methods:** Check for firehose/kinesis in names

## Files to Review

After configuration, suggest user review:
- `bundle.json` - Verify all paths, names, methods, and dependencies
- `transformations/` structure - Verify proper organization
- Transform files - Verify SQL prefixes and metadata cleanup
- `sample_data.json` files - Verify single object format
- Primary dashboard variables - Verify NO prefix on summary table vars
- Other dashboard variables - Verify WITH prefix on summary table vars
- Summary SQL files - Verify template variables used

---

**End of process. Ask user if they want to test deployment or make any adjustments.**
