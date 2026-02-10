---
name: configure-bundle
description: Configure a Hydrolix integration deployment bundle with proper template variables, dashboard structure, and bundle.json. Use when the user wants to set up or fix a Hydrolix integration bundle.
user-invocable: true
---

# Configure Hydrolix Integration Bundle

You are helping configure a Hydrolix integration deployment bundle. Follow this process systematically:

## Phase 1: Discovery and Assessment

1. **Identify the bundle directory**

**Step 1a: Detect current location**
- Check if current directory contains bundle structure (dashboards/, transformations/)
- Check if current directory is a repo root (contains aws/, trafficpeak/ subdirectories)

**Step 1b: If in repo root, prompt for bundle selection**

If current directory contains `aws/` or `trafficpeak/` subdirectories:
- List available bundles by scanning subdirectories
- Present options to user with AskUserQuestion:
  ```
  Which bundle would you like to configure?
  Options:
  - aws/firehose-waf-regional
  - aws/firehose-waf-global
  - aws/bot-detection
  - aws/cdn-insights
  - trafficpeak/security
  - trafficpeak/default_shared
  - [Other - specify path]
  ```
- Store selected bundle path

**Step 1c: If in bundle directory, confirm**
- If current directory has bundle structure, display: "Detected bundle directory: {path}"
- Ask user to confirm or specify different path

**Step 1d: Validate bundle directory exists**
- Navigate to the selected/confirmed bundle directory
- Verify it exists and is accessible
- ❌ BLOCKER if directory doesn't exist

2. **Scan bundle structure**
   - List all files and folders found
   - Identify: dashboards/, summaries/, transformations/ (or transforms/), functions/
   - Determine bundle location: `aws/` or `trafficpeak/` (from path)

3. **Check what exists:**
   - ✓ bundle.json (if missing, needs to be created)
   - ✓ Dashboard JSON files in dashboards/
   - ✓ Summary SQL files in summaries/ (optional)
   - ✓ Transformation and sample data in transformations/ (or transforms/)
   - ✓ Function definitions in functions/ (optional)

4. **Identify the source/vendor:**
   - Check directory name or existing files for clues
   - Ask user for: source name, bundle name, table name, maintainer email
   - Table name is typically "logs", "events", "siem", etc.

## Phase 2: Validation - Collect All Blockers

**IMPORTANT:** Before making any changes, scan all assets and collect ALL blocking issues. Report all errors together for better user experience.

### Validation Checklist:

**1. Check for transform files:**
- ✓ At least one transform file exists in `transformations/` or `transforms/`
- ❌ BLOCKER: No transform files found

**2. For EACH transform file found, validate:**

**a) File is valid JSON:**
- Try to parse the JSON
- ❌ BLOCKER: `{filename}` - Invalid JSON syntax, cannot parse

**b) Has required transform structure:**
- Has `"settings"` object
- Has `"output_columns"` array
- Has `"name"` field
- ❌ BLOCKER: `{filename}` - Missing required transform structure (settings, output_columns, or name)

**c) Has sample_data field:**
- Check if `sample_data` field exists in transform
- ❌ BLOCKER: `{filename}` - Missing sample_data field (provider must add sample data)

**d) Sample data has content:**
- If sample_data field exists, check it's not null/empty
- ❌ BLOCKER: `{filename}` - sample_data field is empty

**3. Check for TrafficPeak + Firehose violation:**

**IF** bundle path contains `trafficpeak/`:
- Scan bundle.json (if exists) for `"method": "firehose"`
- Scan all transform files for `"method": "firehose"`
- ❌ BLOCKER: TrafficPeak bundles cannot use firehose method (only http_streaming allowed)

**4. Check for dashboard files:**
- At least one `.json` file exists in `dashboards/` folder
- ❌ BLOCKER: No dashboard files found

**5. Validate dashboard JSON:**
- For each dashboard, try to parse JSON
- ❌ BLOCKER: `dashboards/{filename}` - Invalid JSON syntax, cannot parse

### Error Reporting:

**If ANY blockers found:**

```
❌ Cannot proceed with bundle configuration. Found {N} blocking issues:

Transform Issues:
1. transformations/transform_1.json - Missing sample_data field
2. transformations/transform_2.json - Invalid JSON syntax, cannot parse
3. transformations/transform_3.json - sample_data field is empty

Bundle Rule Violations:
4. TrafficPeak bundles cannot use firehose method (found in bundle.json)

Dashboard Issues:
5. dashboards/main.json - Invalid JSON syntax, cannot parse

Action Required:
- Contact the team/provider that supplied these bundle assets
- Fix the issues listed above
- Run the configuration skill again once fixes are complete

The bundle validator will perform additional checks after configuration is complete.
```

**STOP HERE** - Do not proceed to Phase 3 until all blockers are resolved.

**If NO blockers found:**
- Report: ✅ All validation checks passed
- Proceed to Phase 3

## Phase 3: Transform Organization and Cleanup

This phase normalizes and organizes all transformation files.

### 3a. Normalize Folder Name

**Check transformation folder name:**
- Look for folders: `transforms/` or `transformations/`
- Identify by checking for JSON files with transform structure:
  - Contains `"settings"` object
  - Contains `"output_columns"` array
  - Contains `"name"` field

**Action:**
- If folder is named `transforms/` (singular) → rename to `transformations/`
- Update any bundle.json references to the folder

### 3b. Organize Transform Files

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

### 3c. Clean Transform Metadata Fields

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

### 3d. Extract and Validate Sample Data

**For EACH transform file:**

**Note:** Phase 2 validation already confirmed sample_data field exists and has content. This phase normalizes the format.

1. **Extract sample_data from transform:**
   - Read the `sample_data` field value

2. **Validate and normalize format:**
   - If sample_data is an array: `[{...}, {...}]`
     - Take ONLY the first element: `[0]`
     - Result should be single object: `{...}`
   - If sample_data is already an object: `{...}`
     - Use as-is ✅
   - **REQUIRED:** Final format must be single object, NOT array

3. **Create sample_data.json file:**
   - Write the normalized sample data object to `sample_data.json`
   - Location:
     - Single transform: `transformations/sample_data.json`
     - Multiple transforms: `transformations/{provider}/sample_data.json`

4. **Update transform file:**
   - Replace the `sample_data` field in transform.json with the normalized object
   - Both files must match exactly

**Result:** Both `sample_data.json` and transform's `sample_data` field contain identical single object.

### 3e. Analyze SQL Transform and Fix Prefixes

**For EACH transform file (transform.json), analyze the `sql_transform` field:**

1. **Determine correct prefix based on bundle location:**
   - If bundle path contains `aws/` → correct prefix = `commons`
   - If bundle path contains `trafficpeak/` → correct prefix = `akamai`

2. **Parse sql_transform field to find:**

   **Function calls pattern:** `(reference|commons|akamai|[a-z_]+)_([a-z_]+)\(`
   - Examples: `reference_breadcrumbs(`, `akamai_city_name(`, `commons_edge_worker(`

   **Dictionary calls pattern:** `dictGet\('(reference|commons|akamai|[a-z_]+)_([a-z_]+)'`
   - Examples: `dictGet('reference_ua_cat_dict'`, `dictGet('commons_geoip_asn_blocks_ipv4'`

3. **Extract base names from the sql_transform field** (without prefix):
   - `reference_breadcrumbs` → base: `breadcrumbs`
   - `commons_ua_cat_dict` → base: `ua_cat_dict`
   - `akamai_city_name` → base: `city_name`
   - Extract from ANY function/dictionary matching the pattern, regardless of prefix

4. **Replace prefixes in sql_transform:**
   - Replace ALL instances of `(reference|commons|akamai)_` with `{correct_prefix}_`
   - Examples for trafficpeak bundle:
     - `reference_breadcrumbs(` → `akamai_breadcrumbs(`
     - `dictGet('reference_ua_cat_dict'` → `dictGet('akamai_ua_cat_dict'`
     - `commons_city_name(` → `akamai_city_name(`

5. **Collect unique base names for bundle.json:**
   - Scan ONLY the `sql_transform` field for ALL function calls and dictionary lookups
   - Extract the unique base names (without prefixes) from what you find
   - **IMPORTANT:** Ignore any `functions/` or `dictionaries/` folders in the bundle - only scan the SQL
   - There may be many functions and dictionaries - collect all of them
   - These base names will populate `shared_functions` and `shared_dictionaries` in bundle.json

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

## Phase 4: Create/Update bundle.json

Now that transforms are analyzed and cleaned, create or update bundle.json with complete information.

### 4a. Determine Bundle Method

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

### 4a-1. Add Method Overrides for Firehose (if applicable)

**IMPORTANT:** Only add method_overrides if firehose method is detected.

**Check if firehose is used:**
- Bundle-level method = `firehose`, OR
- Any transform has method = `firehose`

**If NO firehose detected:**
- Skip this section, do not add method_overrides

**If firehose IS detected:**

**Step 1: Determine the type of firehose bundle**

Check bundle name, source, or transform names:

**Type A: WAF Logs**
- Bundle/source name contains "waf" (case insensitive)
- OR transform path contains "waf"
- Examples: `firehose-waf-regional`, `aws-waf-logs`

**Type B: CloudFront**
- Bundle/source name contains "cloudfront" (case insensitive)
- OR transform path contains "cloudfront"
- OR ui.source.full_title contains "CloudFront"
- Examples: `firehose-waf-global`, `cloudfront_firehose`

**Type C: Other Firehose**
- Neither WAF nor CloudFront
- Examples: generic firehose transforms

**Step 2: Add method_overrides based on type**

**For Type A (WAF Regional):**
```json
"method_overrides": {
  "stream_prefix": "aws-waf-logs-hdx"
}
```

**For Type B (CloudFront Global):**
```json
"method_overrides": {
  "region": "us-east-1",
  "stream_prefix": "aws-waf-logs-hdx"
}
```

**For Type C (Other):**
- Do not add method_overrides

**Placement:**
- Add method_overrides at the bundle level (same level as "method", "name", "source")

### 4b. Build Transform References

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

### 4c. Populate Dependencies

Using the base names collected from Phase 3e:

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

**IMPORTANT - Dependency Protocol:**

1. **All functions and dictionaries found in the sql_transform go into `shared_functions` and `shared_dictionaries`**
   - These are "shared" because we assume they will already exist on the target cluster when the bundle is deployed
   - The bundle.json is declaring: "This bundle requires these functions/dictionaries to exist on the cluster"
   - There may be many functions and dictionaries - include all base names found in the SQL

2. **Always leave `required_functions` and `required_dictionaries` as empty arrays `[]`**
   - These fields are not used in the current bundle configuration workflow

3. **Ignore the `functions/` and `dictionaries/` folders**
   - Even if the bundle contains a `functions/` or `dictionaries/` directory, do NOT use these to populate dependencies
   - Only scan the `sql_transform` field to determine what functions/dictionaries are actually used
   - These folders may contain extra files that aren't referenced in the transform

4. **Validation happens separately**
   - A separate validator tool will check if the declared functions/dictionaries exist on the target cluster
   - Missing ones can be added manually by the operator

**Note:** List base names only (without prefixes). The actual prefixes are in the sql_transform.

### 4d. Complete bundle.json Structure

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
      "shared_dictionaries": [/* from Phase 3e */],
      "shared_functions": [/* from Phase 3e */]
    }
  },
  "metadata": {
    "channel_type": "AWS",
    "description": "{Bundle Description}",
    "maintainer": "{email}",
    "version": "1.0.0"
  },

  NOTE: channel_type valid values are: "AWS" | "Azure" | "GCP" | "3rdParty" | "Internal"
        Default to "AWS" for most bundles unless specifically required otherwise.
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

## Phase 5: Fix Summary SQL Files

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

## Phase 6: Fix Dashboard Structure

For EACH dashboard JSON file:

### 6a. Fix Dashboard Wrapper and Structure

**Step 1: Add dashboard wrapper if missing**

Required structure:
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

**Step 2: Populate __elements with datasource model**

The `__elements` object must contain the datasource model:
```json
"__elements": {
  "model": {
    "datasource": {
      "type": "hydrolix-hydrolix-datasource",
      "uid": "__DATASOURCE__"
    }
  }
}
```

If `__elements` is empty (`{}`), populate it with this structure.

**Step 3: Remove __inputs array**

If the dashboard contains an `__inputs` array (typically after `__elements`), **remove it entirely**. This is an artifact from dashboard export that should not be in the final bundle.

Example of what to remove:
```json
"__inputs": [
  {
    "name": "DS_HYDROLIX-HYDROLIX-DATASOURCE",
    "pluginId": "hydrolix-hydrolix-datasource",
    ...
  }
]
```

The `__requires` array should remain - only remove `__inputs`.

### 6b. Update Dashboard UID
Find the UID at the bottom of the dashboard:
- Replace hardcoded UID → `"uid": "__DASHBOARD_UUID__"`

### 6c. Fix Template Variables

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

**REQUIRED: Add raw_table variable for validation**

The validator requires `__PROJECT_NAME__` to appear somewhere in the dashboard. Add this hidden variable to `templating.list`:

```json
{
  "current": {
    "selected": false,
    "text": "__PROJECT_NAME__.__TABLE_NAME__",
    "value": "__PROJECT_NAME__.__TABLE_NAME__"
  },
  "hide": 2,
  "name": "raw_table",
  "options": [
    {
      "selected": false,
      "text": "__PROJECT_NAME__.__TABLE_NAME__",
      "value": "__PROJECT_NAME__.__TABLE_NAME__"
    }
  ],
  "query": "__PROJECT_NAME__.__TABLE_NAME__",
  "skipUrlSync": true,
  "type": "constant"
}
```

This variable is hidden (hide: 2) and ensures the validator can find the required `__PROJECT_NAME__` pattern.

### 6d. Update Datasource UIDs Throughout Dashboard

**Replace all datasource UID references with the template variable:**

Dashboards exported from Grafana contain hardcoded datasource UIDs like:
- `"uid": "${DS_HYDROLIX-HYDROLIX-DATASOURCE}"`
- `"uid": "beydc3kqc3ksge"` (or other random UIDs)

These must ALL be replaced with: `"uid": "__DATASOURCE__"`

**Where to find these UIDs:**
1. **Panel datasources** - In each panel's `datasource` object
2. **Target datasources** - In each panel's `targets[].datasource` object
3. **Template variable datasources** - In adhoc filter variables (already handled in 6c)

**Example replacements:**

Before:
```json
"datasource": {
  "type": "hydrolix-hydrolix-datasource",
  "uid": "${DS_HYDROLIX-HYDROLIX-DATASOURCE}"
}
```

After:
```json
"datasource": {
  "type": "hydrolix-hydrolix-datasource",
  "uid": "__DATASOURCE__"
}
```

**Important:** Keep the `"type": "hydrolix-hydrolix-datasource"` field - only replace the UID value.

**How to do this efficiently:**
- Use a global find/replace across the dashboard JSON file
- Search for: `"uid": "${DS_HYDROLIX-HYDROLIX-DATASOURCE}"`
- Replace with: `"uid": "__DATASOURCE__"`
- Also search for any other hardcoded datasource UIDs and replace them

## Phase 7: Update bundle.json with Dashboard Paths

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

## Phase 8: Configuration Summary

After making all changes, provide a summary:

```
✅ Phase 2 Validation: All blockers resolved
   - {N} transform files validated
   - {N} dashboard files validated
   - All JSON files parseable
   - All required fields present
   - No TrafficPeak + firehose violations

✅ Transform Organization Complete (Phase 3):
   - Folder normalized to: transformations/
   - Transform count: {N} ({single/multiple})
   - Structure: {single transform.json OR subdirectories per provider}
   - Metadata fields cleaned: uuid, created, modified, url, table
   - Sample data validated: single object format
   - SQL prefixes replaced: {old_prefix}_ → {correct_prefix}_

✅ Created/Updated Files (Phase 4-7):
   - bundle.json (with correct method, method_overrides if firehose, and dependencies)
   - transformations/{structure}
   - summaries/{files}.sql (template variables)
   - dashboards/{primary}.json (primary)
   - dashboards/{other}.json (other dashboards)

✅ Dependencies Populated (scanned from sql_transform):
   - shared_functions: [{list}]
   - shared_dictionaries: [{list}]
   - All prefixes corrected for {aws/trafficpeak} bundle
   - Note: functions/ and dictionaries/ folders ignored per protocol

✅ Template Variables Configured:
   - __PROJECT_NAME__ → project name
   - __TABLE_NAME__ → base table name
   - __SUMMARY_TABLE_NAME_X__ → summary tables
   - __DATASOURCE__ → datasource UID
   - __DASHBOARD_UUID__ → generated UUID

✅ Key Patterns Applied:
   - Transform methods: {http_streaming/firehose/kinesis/multi_stream}
   - Method overrides: {added for WAF/CloudFront firehose if applicable}
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

## Common Issues and How They're Handled

**Phase 2 Blockers (Cannot proceed):**
1. **Missing sample data:** Transform files must have sample_data field - BLOCKER
2. **Invalid JSON:** Any JSON file that can't be parsed - BLOCKER
3. **TrafficPeak + Firehose:** TrafficPeak bundles cannot use firehose method - BLOCKER
4. **Missing required structure:** Transform missing settings/output_columns/name - BLOCKER

**Auto-Fixed Issues (Skill handles):**
1. **Array-wrapped sample data:** Skill extracts first object from array
2. **Wrong SQL prefixes:** Skill replaces reference/commons/akamai with correct prefix
3. **Missing dashboard wrapper:** Skill adds `"dashboard": { }` wrapper
4. **Old ${VAR_*} variables:** Skill replaces with template variables
5. **Wrong folder name:** Skill renames `transforms/` to `transformations/`
6. **Transform metadata:** Skill removes uuid, created, modified, url, table fields
7. **Datasource UIDs:** Skill replaces with `__DATASOURCE__`
8. **Dashboard UID:** Skill replaces with `__DASHBOARD_UUID__`
9. **Missing method_overrides:** Skill adds for WAF/CloudFront firehose bundles

**Dashboard Variable Patterns (Applied by skill):**
- Primary dashboard summary vars: `__SUMMARY_TABLE_NAME_X__` (no prefix)
- Other dashboard summary vars: `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_X__` (with prefix)
- All dashboard table vars: `__PROJECT_NAME__.__TABLE_NAME__`

## Files to Review

After configuration, suggest user review:
- `bundle.json` - Verify all paths, names, methods, method_overrides (if firehose), and dependencies
- `transformations/` structure - Verify proper organization and naming
- Transform files - Verify SQL prefixes match bundle location (commons/akamai) and metadata cleaned
- `sample_data.json` files - Verify single object format (not array)
- Primary dashboard variables - Verify NO prefix on summary table vars
- Other dashboard variables - Verify WITH prefix on summary table vars
- All dashboards - Verify datasource UIDs are `__DATASOURCE__` and dashboard UID is `__DASHBOARD_UUID__`
- Summary SQL files - Verify template variables used (`__PROJECT_NAME__.__TABLE_NAME__`)

---

**End of process. Ask user if they want to test deployment or make any adjustments.**
