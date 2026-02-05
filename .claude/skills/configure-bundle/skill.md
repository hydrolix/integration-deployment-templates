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

2. **Check what exists:**
   - ✓ bundle.json (if missing, needs to be created)
   - ✓ Dashboard JSON files in dashboards/
   - ✓ Summary SQL files in summaries/ (optional)
   - ✓ Transformation and sample data in transformations/
   - ✓ Function definitions in functions/

3. **Identify the source/vendor:**
   - Check directory name or existing files for clues
   - Ask user for: source name, bundle name, table name, maintainer email
   - Table name is typically "logs", "events", "siem", etc.

## Phase 2: Create/Update bundle.json

If bundle.json doesn't exist, create it with this structure:

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
      "shared_dictionaries": [],
      "shared_functions": []
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
  "method": "http_streaming",
  "name": "{source}_{bundle_name}",
  "other_dashboards": [],
  "solution": true,
  "source": "{source}",
  "summary_tables": [],
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
  ],
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
- Shared functions should be listed in `dependencies.hydrolix.shared_functions`
- `tables[].name` should be set to the table name provided by the user (e.g., "logs", "events", "siem")

## Phase 3: Fix Sample Data Files

Check and fix sample data in `transformations/`:

1. **Check sample_data.json file:**
   - Read the first character of the file
   - If it starts with `[`, it's incorrectly wrapped in an array
   - **REQUIRED FORMAT:** Sample data must be a single object `{...}`, NOT an array `[{...}]`

2. **Fix array-wrapped sample_data.json:**
   - Remove the array wrapper `[...]`
   - Keep only the object inside `{...}`
   - Example transformation:
     ```
     WRONG: [{...}]
     RIGHT: {...}
     ```

3. **Check transform.json sample_data field:**
   - Verify the `sample_data` field inside `transform.json` is an object `{...}`, not an array
   - This should match the structure of the standalone `sample_data.json` file

4. **Why this matters:**
   - The deployment system expects sample data to be a single object
   - Array-wrapped sample data will cause validation or deployment failures
   - Both files must have consistent structure

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

### 5a. Fix Dashboard Wrapper and Structure

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

### 5d. Update Datasource UIDs Throughout Dashboard

**Replace all datasource UID references with the template variable:**

Dashboards exported from Grafana contain hardcoded datasource UIDs like:
- `"uid": "${DS_HYDROLIX-HYDROLIX-DATASOURCE}"`
- `"uid": "beydc3kqc3ksge"` (or other random UIDs)

These must ALL be replaced with: `"uid": "__DATASOURCE__"`

**Where to find these UIDs:**
1. **Panel datasources** - In each panel's `datasource` object
2. **Target datasources** - In each panel's `targets[].datasource` object
3. **Template variable datasources** - In adhoc filter variables (already handled in 5c)

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

## Phase 6: Update bundle.json with Dashboard Paths

1. **Choose the primary dashboard** (usually the main overview/analysis dashboard)
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

## Phase 7: Validation Summary

After making all changes, provide a summary:

```
✅ Created/Updated Files:
   - bundle.json
   - transformations/sample_data.json (removed array wrapper if present)
   - transformations/transform.json (verified sample_data field)
   - summaries/{files}.sql (template variables)
   - dashboards/{primary}.json (primary - no prefix on summary vars)
   - dashboards/{other}.json (other - WITH prefix on summary vars)

✅ Template Variables Configured:
   - __PROJECT_NAME__ → project name
   - __TABLE_NAME__ → base table name
   - __SUMMARY_TABLE_NAME_1__ → summary table 1
   - __SUMMARY_TABLE_NAME_2__ → summary table 2
   - __DATASOURCE__ → datasource UID
   - __DASHBOARD_UUID__ → generated UUID

✅ Key Patterns Applied:
   - Primary dashboard: __SUMMARY_TABLE_NAME_X__ (no prefix)
   - Other dashboards: __PROJECT_NAME__.__SUMMARY_TABLE_NAME_X__ (with prefix)
   - Regular tables: __PROJECT_NAME__.__TABLE_NAME__ (all dashboards)

⚠️ Important Notes:
   - ui.source.full_title must be unique across all bundles
   - Primary dashboard gets different variable processing than others
   - Sample data must be a single object, not an array
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
2. **Duplication:** If seeing `project.project.table`, primary dashboard has wrong pattern (should use `__SUMMARY_TABLE_NAME_X__` without prefix)
3. **Missing project:** If seeing just `table_name`, other dashboard missing `__PROJECT_NAME__.` prefix
4. **Syntax errors in queries:** Old `${VAR_*}` variables not replaced
5. **Missing wrapper:** Dashboard content not wrapped in `"dashboard": { }`

## Files to Review

After configuration, suggest user review:
- `bundle.json` - Verify all paths, names, and metadata
- `sample_data.json` - Verify it's an object, not an array
- `transform.json` - Verify sample_data field is an object
- Primary dashboard variables - Verify NO prefix on summary table vars
- Other dashboard variables - Verify WITH prefix on summary table vars
- Summary SQL files - Verify template variables used

---

**End of process. Ask user if they want to test deployment or make any adjustments.**
