---
name: bundle-checklist
description: Show a quick reference checklist for Hydrolix bundle configuration. Use when the user needs a quick reminder of bundle configuration patterns.
user-invocable: true
---

# Hydrolix Bundle Configuration - Quick Reference

## Quick Checklist

When configuring a Hydrolix integration bundle:

### 1. Files Structure
```
source/bundle_name/
├── bundle.json ← CREATE THIS
├── dashboards/
│   ├── primary_dashboard.json ← FIX: wrapper, UID, vars (NO PREFIX)
│   ├── other_dashboard_1.json ← FIX: wrapper, UID, vars (WITH PREFIX)
│   └── other_dashboard_2.json ← FIX: wrapper, UID, vars (WITH PREFIX)
├── summaries/
│   ├── summary1.sql ← FIX: hardcoded tables → __PROJECT_NAME__.__TABLE_NAME__
│   └── summary2.sql ← FIX: hardcoded tables → __PROJECT_NAME__.__TABLE_NAME__
├── transformations/
│   ├── transform.json
│   └── sample_data.json
└── functions/
    ├── function1.json
    └── function2.json
```

### 2. Dashboard JSON Fixes

#### All Dashboards
- [ ] Wrapped in `{"dashboard": { ... }}`
- [ ] UID changed to `"uid": "__DASHBOARD_UUID__"`
- [ ] Replaced `${VAR_TIMESTAMP}` with `timestamp`
- [ ] Table var uses: `__PROJECT_NAME__.__TABLE_NAME__`

#### Primary Dashboard Summary Table Vars
```json
"query": "__SUMMARY_TABLE_NAME_1__"
```
**NO** `__PROJECT_NAME__.` prefix!

#### Other Dashboards Summary Table Vars
```json
"query": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"
```
**WITH** `__PROJECT_NAME__.` prefix!

### 3. Common Mistakes

❌ **Wrong:** Primary dashboard with prefix
```json
"query": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"
```
Result: `project.project.summary_table` (duplicated!)

✅ **Correct:** Primary dashboard without prefix
```json
"query": "__SUMMARY_TABLE_NAME_1__"
```
Result: `project.summary_table`

---

❌ **Wrong:** Other dashboard without prefix
```json
"query": "__SUMMARY_TABLE_NAME_1__"
```
Result: `summary_table` (missing project!)

✅ **Correct:** Other dashboard with prefix
```json
"query": "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"
```
Result: `project.summary_table`

### 4. Variable Reference

| What | Primary | Other | Why Different? |
|------|---------|-------|----------------|
| Tables | `__PROJECT_NAME__.__TABLE_NAME__` | `__PROJECT_NAME__.__TABLE_NAME__` | Same for both |
| Summary Tables | `__SUMMARY_TABLE_NAME_X__` | `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_X__` | Different processing in validator |

### 5. bundle.json Essentials

```json
{
  "dashboard": {
    "path": "dashboards/PRIMARY.json"  ← Choose main dashboard
  },
  "other_dashboards": [
    {"path": "dashboards/OTHER1.json"},
    {"path": "dashboards/OTHER2.json"}
  ],
  "tables": [{
    "dashboard_var": "__TABLE_NAME__",
    "name": "actual_table_name"
  }],
  "summary_tables": [{
    "dashboard_var": "__SUMMARY_TABLE_NAME_1__",
    "name": "actual_summary_name",
    "parent_table_name": "parent_table",
    "sql": {"path": "summaries/file.sql"}
  }],
  "ui": {
    "source": {
      "full_title": "UNIQUE_NAME"  ← Must be unique!
    }
  }
}
```

### 6. Deployment Test Commands

After configuration:
```bash
# Validate bundle
cd integration-deployment-templates
cargo run -- validate source/bundle_name

# Deploy bundle (if validation passes)
cargo run -- deploy source/bundle_name
```

### 7. Troubleshooting

**Error:** `Table _local.summary_table does not exist`
- ❌ Other dashboard missing `__PROJECT_NAME__.` prefix

**Error:** `Table project.project.summary_table does not exist`
- ❌ Primary dashboard has `__PROJECT_NAME__.` prefix (should not!)

**Error:** `Syntax error: failed at position X ('WHERE')`
- ❌ Table variable is empty or not substituted
- ❌ Check `${VAR_*}` variables replaced with proper templates

**Error:** `failed at position X ('$')`
- ❌ Old `${VAR_*}` variables still in dashboard

---

## Quick Command

For full guided process, use: `/configure-bundle [path]`
