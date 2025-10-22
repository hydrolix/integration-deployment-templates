# Bundle Deployer - TypeScript/Deno

A comprehensive validation and deployment tool for Hydrolix integration bundles. Validates bundle structure, deploys to Hydrolix clusters, creates Grafana dashboards and alerts, and performs automated testing.

## Features

✅ **Complete Bundle Validation** - 11 validation modules checking structure, naming, checksums, and dependencies  
✅ **Hydrolix Deployment** - Tables, transforms, summary tables, functions, and dictionaries  
✅ **Grafana Integration** - Dashboards, datasources, and alert rules  
✅ **Alert Rules Support** - Grafana 12+ compatible with automatic format conversion  
✅ **Dictionary Support** - CSV, YAML/Regexp, and JSON definition formats  
✅ **Custom Functions** - SQL function deployment with automatic prefixing  
✅ **Headless Browser Testing** - Automated dashboard validation with Puppeteer  
✅ **Template Variables** - Automatic replacement of project names, datasources, and table names  
✅ **Smart Dependency Handling** - Creates bundle-specific resources, validates infrastructure resources  

## Quick Start

### Prerequisites

- [Deno](https://deno.land/manual/getting_started/installation) installed
- Docker (for local Grafana testing)
- Hydrolix cluster credentials

### Environment Variables

```bash
export BUNDLE_TESTING_CLUSTER="your-cluster.hydrolix.live"
export BUNDLE_TESTING_USERNAME="your-username"
export BUNDLE_TESTING_PASSWORD="your-password"
```

### Installation

```bash
# Clone the repository
cd bundle-deployer

# Check TypeScript compilation
deno check src/main.ts
```

### Usage

```bash
# Validate all bundles (no deployment)
deno run --allow-all src/main.ts

# Validate specific bundle by name
deno run --allow-all src/main.ts mcdn_test

# Deploy to local Grafana with full testing
deno run --allow-all src/main.ts --local mcdn_test

# Deploy dashboard only (faster, no table creation)
deno run --allow-all src/main.ts --local-dashboard-only mcdn_test

# Production validation mode (check dependencies exist)
deno run --allow-all src/main.ts --production mcdn_test

# Scan WIP directory for bundles
deno run --allow-all src/main.ts --wip
```

## Project Structure

```
bundle-deployer/
├── src/
│   ├── main.ts                          # Entry point, CLI handling
│   ├── types/
│   │   └── bundle.ts                    # TypeScript interfaces & validation
│   ├── validation/                      # Validation modules
│   │   ├── naming_is_valid.ts           # Naming conventions
│   │   ├── dashboard_is_valid.ts        # Dashboard structure
│   │   ├── alert_rules_are_valid.ts     # Alert rules format
│   │   ├── transforms_are_valid.ts      # Transform schemas
│   │   ├── no_duplicate_tokens.ts       # Unique template vars
│   │   ├── no_bad_checksums.ts          # SHA256 verification
│   │   ├── sample_data_exists.ts        # Sample data validation
│   │   ├── summary_table.ts             # Summary table SQL
│   │   ├── valid_base_url.ts            # URL validation
│   │   ├── no_global_duplicates.ts      # Cross-bundle checks
│   │   └── check_dependencies.ts        # Dependency validation
│   ├── grafana/
│   │   ├── container.ts                 # Docker management
│   │   └── interface.ts                 # Grafana API client
│   ├── deploy.ts                        # Main deployment orchestration
│   ├── deploy_only_dashboard.ts         # Dashboard-only deployment
│   ├── hdx.ts                           # Hydrolix API client
│   ├── hdx_check_dependencies.ts        # Production dependency checks
│   ├── headless_browser.ts              # Puppeteer browser testing
│   └── utils/
│       └── error.ts                     # Error handling utilities
└── my-bundles/                          # Bundle definitions
    └── your_bundle/
        ├── bundle.json                  # Bundle configuration
        ├── dashboards/                  # Grafana dashboards & alerts
        ├── transformations/             # Hydrolix transform schemas
        ├── summaries/                   # Summary table SQL
        └── dictionaries/                # Dictionary data files
```

## Bundle Structure

### Minimum bundle.json

```json
{
  "name": "my_integration",
  "source": "cdn",
  "method": "http_streaming",
  "beta": false,
  "base_url": "https://github.com/org/repo/blob/main/bundle",
  "dashboard": {
    "path": "dashboards/main.json",
    "project_var": "__PROJECT_NAME__"
  },
  "tables": [
    {
      "name": "logs",
      "dashboard_var": "__TABLE_NAME__",
      "transforms": [
        { "path": "transformations/parser.json" }
      ]
    }
  ],
  "ui": {
    "primary_url": "https://docs.example.com",
    "method": {
      "full_title": "HTTP Streaming",
      "icon_url": "https://example.com/icon.png"
    },
    "source": {
      "full_title": "My Service",
      "icon_url": "https://example.com/logo.png"
    },
    "data_category": "cdn"
  },
  "metadata": {
    "version": "1.0.0",
    "maintainer": "you@example.com",
    "description": "My integration bundle",
    "channel_type": "AWS"
  }
}
```

### With Dependencies

```json
{
  "dependencies": {
    "hydrolix": {
      "required_functions": [
        {
          "name": "parse_user_agent",
          "description": "Extract browser from UA",
          "sql": "(ua) -> extract(ua, '([A-Za-z]+)/')"
        }
      ],
      "required_dictionaries": [
        {
          "name": "ua_categories",
          "source": "dictionaries/ua_categories.yaml"
        }
      ]
    }
  }
}
```

### With Alert Rules

```json
{
  "alert_rules": {
    "path": "dashboards/alerts.json"
  }
}
```

### With Summary Tables

```json
{
  "summary_tables": [
    {
      "name": "logs_summary_min",
      "parent_table_name": "logs",
      "dashboard_var": "__SUMMARY_TABLE_1__",
      "sql": {
        "path": "summaries/minute_rollup.sql"
      }
    }
  ]
}
```

## Template Variables

### Supported Variables

All dashboard, alert, and SQL files support these template variables:

- `__PROJECT_NAME__` - Replaced with Hydrolix project name (e.g., `sample_project`)
- `__DATASOURCE__` - Replaced with Grafana datasource UID
- `__DASHBOARD_UUID__` - Replaced with created dashboard UID
- `__TABLE_NAME__` - Replaced with table name (defined per table in bundle.json)
- `__SUMMARY_TABLE_1__` - Replaced with summary table name (numbered per summary)

### Example Usage

**In dashboard JSON:**
```json
{
  "datasource": {
    "uid": "__DATASOURCE__"
  },
  "targets": [{
    "rawSql": "SELECT * FROM __PROJECT_NAME__.logs WHERE $__timeFilter(timestamp)"
  }]
}
```

**In alert rules:**
```json
{
  "dashboardUid": "__DASHBOARD_UUID__",
  "data": [{
    "datasourceUid": "__DATASOURCE__",
    "model": {
      "rawSql": "SELECT count(*) FROM __PROJECT_NAME__.__TABLE_NAME__"
    }
  }]
}
```

**Replacement happens automatically during deployment!**

## Dictionary Support

### Three Dictionary Types

#### 1. CSV Dictionaries (Simple Lookups)

**File:** `dictionaries/countries.csv`
```csv
country_code,country_name
US,United States
UK,United Kingdom
```

**Bundle.json:**
```json
{
  "name": "country_lookup",
  "source": "dictionaries/countries.csv"
}
```

**Behavior:**
- Uploads CSV file to Hydrolix
- Auto-detects columns from header row
- Creates dictionary with `complex_key_hashed` layout
- Uses first column as primary key

#### 2. YAML/Regexp Dictionaries (Pattern Matching)

**File:** `dictionaries/user_agents.yaml`
```yaml
- regexp: '(?i).*chrome.*'
  browser: 'Chrome'
  is_mobile: 0
- regexp: '(?i).*firefox.*'
  browser: 'Firefox'
  is_mobile: 0
```

**Bundle.json:**
```json
{
  "name": "ua_parser",
  "source": "dictionaries/user_agents.yaml"
}
```

**Behavior:**
- Uploads YAML file to Hydrolix
- Auto-detects attributes from YAML keys
- Creates dictionary with `regexp_tree` layout
- Uses `regexp` as primary key
- Sets format to `Regexp` with `dictionary_load_level: ["ALL"]`

#### 3. JSON Definitions (Pre-Configured)

**File:** `dictionaries/geoip_blocks.json`
```json
{
  "name": "geoip_city_blocks_ipv4",
  "settings": {
    "filename": "geoip_data.csv",
    "layout": "ip_trie",
    "format": "CSVWithNames",
    "output_columns": [...],
    "primary_key": ["network"]
  }
}
```

**Bundle.json:**
```json
{
  "name": "geoip_city_blocks",
  "source": "dictionaries/geoip_blocks.json"
}
```

**Behavior:**
- Loads JSON (complete API payload)
- POSTs directly to Hydrolix API
- No file upload or parsing needed
- **Requires referenced data file to exist on cluster**

### Dictionary Auto-Detection

The tool automatically determines dictionary type by file extension:
- `.csv` → CSV dictionary
- `.yaml` or `.yml` → Regexp dictionary
- `.json` → Pre-configured definition

### Smart Dependency Handling

**For each dictionary:**
1. ✓ Check if already exists on cluster → Skip if found
2. ✓ Check if local file exists → Create if found
3. ✓ Warn if missing → Continue anyway (might be infrastructure)

This handles both:
- **Bundle-specific dictionaries** (small, included in bundle)
- **Infrastructure dictionaries** (large, pre-loaded by ops team)

## Function Support

### Custom SQL Functions

**Bundle.json:**
```json
{
  "dependencies": {
    "hydrolix": {
      "required_functions": [
        {
          "name": "extract_domain",
          "description": "Extract domain from URL",
          "sql": "(url) -> domain(url)"
        }
      ]
    }
  }
}
```

**Behavior:**
- Checks if function exists (with project prefix)
- Creates if missing
- Automatically prefixes: `extract_domain` → `sample_project_extract_domain`
- Rewrites all SQL in transforms to use prefixed names

### Automatic SQL Rewriting

**Original transform SQL:**
```sql
SELECT extract_domain(request_url) AS domain FROM {STREAM}
```

**Deployed SQL:**
```sql
SELECT sample_project_extract_domain(request_url) AS domain FROM {STREAM}
```

**This happens automatically for:**
- All functions in `required_functions`
- All dictionaries in `required_dictionaries`
- Done during transform processing

## Alert Rules Support

### Grafana 12+ Compatible

The tool supports Grafana 12's alert rule format with automatic cleanup of UI-only fields.

### Alert Rules File

**File:** `dashboards/alerts.json`
```json
{
  "apiVersion": 1,
  "groups": [
    {
      "name": "critical_alerts",
      "folder": "Solutions",
      "interval": "1m",
      "rules": [
        {
          "uid": "error_rate_alert",
          "title": "High Error Rate",
          "condition": "C",
          "data": [
            {
              "refId": "A",
              "queryType": "table",
              "datasourceUid": "__DATASOURCE__",
              "model": {
                "rawSql": "SELECT error_rate FROM __PROJECT_NAME__.summary"
              }
            },
            {
              "refId": "B",
              "datasourceUid": "__expr__",
              "model": {
                "expression": "A",
                "type": "reduce",
                "reducer": "last"
              }
            },
            {
              "refId": "C",
              "datasourceUid": "__expr__",
              "model": {
                "expression": "B",
                "type": "threshold",
                "conditions": [{
                  "evaluator": {"params": [15], "type": "gt"}
                }]
              }
            }
          ],
          "for": "1m",
          "noDataState": "NoData",
          "execErrState": "Error"
        }
      ]
    }
  ]
}
```

**Bundle.json:**
```json
{
  "alert_rules": {
    "path": "dashboards/alerts.json"
  }
}
```

### Alert Format Requirements (Grafana 12+)

**Required 3-step structure:**
- **Step A**: Query (from datasource)
- **Step B**: Reduce (aggregate to single value)
- **Step C**: Threshold (condition to alert)

Old 2-step format (A → C) will fail!

### Automatic Field Cleanup

The tool automatically removes UI-only fields before deployment:
- `notification_settings`
- `isPaused`
- `templating`
- `meta`
- `pluginVersion`
- `format`
- `editorType`
- `builderOptions`

## CLI Reference

### Commands

```bash
# Validate all bundles
deno run --allow-all src/main.ts

# Validate specific bundle
deno run --allow-all src/main.ts BUNDLE_NAME

# Deploy to local Grafana (full stack)
deno run --allow-all src/main.ts --local BUNDLE_NAME

# Deploy dashboard only (fast testing)
deno run --allow-all src/main.ts --local-dashboard-only BUNDLE_NAME

# Production validation (no creation)
deno run --allow-all src/main.ts --production BUNDLE_NAME

# Scan WIP directory
deno run --allow-all src/main.ts --wip

# Generate traffic output JSON
deno run --allow-all src/main.ts --output BUNDLE_NAME
```

### Flags

- `--local` - Deploy to local Grafana container with full testing
- `--local-dashboard-only` - Deploy only dashboard and alerts (skip tables)
- `--production` - Validate dependencies exist without creating them
- `--wip` - Scan WIP directory instead of root
- `--marketplace` - Special settings for marketplace bundles
- `--output` - Dump deployment output JSON for traffic generation

### Bundle Name Filtering

The tool scans for `bundle.json` files and matches against the `name` field:

```bash
deno run --allow-all src/main.ts mcdn
# Matches: "http_streaming_mcdn_test", "mcdn_analytics", etc.
```

## Deployment Modes

### Sandbox Mode (Default)

**Purpose:** Test bundles in isolated environment

**Behavior:**
- ✅ Creates all resources from scratch
- ✅ Uses local Grafana Docker container
- ✅ Inserts sample data for testing
- ✅ Creates functions and dictionaries from bundle
- ✅ Runs headless browser validation

**Use for:**
- Bundle development
- Pre-deployment validation
- Testing changes

### Production Mode (`--production`)

**Purpose:** Validate bundle is production-ready

**Behavior:**
- ✅ Validates bundle structure
- ✅ Checks dependencies exist (doesn't create)
- ❌ Doesn't deploy anything
- ❌ Doesn't modify cluster

**Use for:**
- CI/CD validation
- Pre-merge checks
- Production deployment verification

## Validation Modules

### Comprehensive Checks

**Structure Validation:**
- ✅ Bundle JSON schema valid
- ✅ All required fields present
- ✅ File paths don't escape bundle directory
- ✅ URLs use HTTPS (or file://)

**Naming Validation:**
- ✅ Names use allowed characters only
- ✅ Template variables in correct format (`__VAR__`)
- ✅ No duplicate template variables
- ✅ No global duplicates across bundles

**File Validation:**
- ✅ Dashboard JSON files exist and parse
- ✅ Alert rules JSON files exist and parse
- ✅ Transform JSON files exist and parse
- ✅ Summary SQL files exist
- ✅ SHA256 checksums match (if provided)

**Content Validation:**
- ✅ Dashboards have required structure
- ✅ Alert rules have required fields (Grafana 12 format)
- ✅ Transforms have valid schemas
- ✅ Sample data files exist

**Dependency Validation:**
- ✅ Required functions declared
- ✅ Required dictionaries declared
- ✅ Functions/dictionaries referenced in SQL are declared

## How It Works

### 1. Bundle Discovery

Scans directories for `bundle.json` files:

```typescript
for await (const entry of walk(searchPath, { maxDepth: 2 })) {
  if (entry.isFile && entry.name === "bundle.json") {
    bundles.push(entry.path);
  }
}
```

**Note:** `maxDepth: 2` means it looks 2 directories deep.

### 2. Validation Pipeline

Each bundle runs through 11 validation modules:

```typescript
valid_base_url.run(base, bundle);           // Check URLs
no_duplicate_tokens.run(bundle);            // Check template vars
naming_is_valid.run(bundle);                // Check naming
await no_bad_checksums.run(base, bundle);   // Verify SHA256
await transforms_are_valid.run(base, bundle); // Validate schemas
await dashboard_is_valid.run(base, bundle); // Check dashboard
await alert_rules_are_valid.run(base, bundle); // Check alerts
await sample_data_exists.run(base, bundle); // Check sample files
summary_table.run(bundle);                  // Validate SQL
await check_dependencies.run(base, bundle); // Check deps
```

**Stops at first error** - fail fast principle.

### 3. Deployment Orchestration

**Order matters!** Dependencies must be created before dependents:

```
1. Functions       (transforms need them)
2. Dictionaries    (transforms need them)
3. Tables          (transforms attach to tables)
4. Transforms      (data needs transform schemas)
5. Sample Data     (populates base tables)
6. Summary Tables  (aggregate base table data)
7. Datasource      (dashboards need datasource)
8. Dashboard       (alerts need dashboard UID)
9. Alert Rules     (reference dashboard)
```

### 4. Template Replacement

**Three stages:**

**Stage 1 - Dashboard:**
```typescript
dashboard = dashboard.replace(/__PROJECT_NAME__/g, "sample_project");
dashboard = dashboard.replace(/__DATASOURCE__/g, "af1njk2miaha8f");
dashboard = dashboard.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());
```

**Stage 2 - Table Variables:**
```typescript
for (const table of bundle.tables) {
  dashboard = dashboard.replace(table.dashboard_var, table.name);
}
```

**Stage 3 - Transform SQL:**
```typescript
sql = sql.replace(/reference_city_name\(/g, "sample_project_reference_city_name(");
sql = sql.replace(/dictGet\('ua_dict'/g, "dictGet('sample_project_ua_dict'");
```

### 5. Automated Testing

**Headless Chrome validates:**
- Dashboard loads without errors
- All panels render
- No datasource errors
- No "No Data" errors

**120 second wait** for all queries to complete.

## Troubleshooting

### Common Issues

**"Module not found"**
- Check import paths are correct
- Verify file exists
- Ensure you're in the right directory (`bundle-deployer/`, not `my-bundles/`)

**"No bundles were checked"**
- Run from `bundle-deployer/` or `my-bundles/` directory
- Check `maxDepth` if bundles are deeply nested
- Verify bundle name matches filter

**"Unknown function sample_project_X"**
- Functions weren't created (check logs for creation errors)
- Function name in SQL doesn't match bundle dependencies
- Check bundle.json uses `"sql"` field (not `"definition"`)

**"Dictionary not found"**
- Dictionary wasn't created (check for warnings)
- Dictionary data file missing (for JSON definitions)
- Dictionary might need to be pre-loaded by infrastructure team

**Alert rule creation fails with "bad request data"**
- Alert format is for old Grafana version (needs 3-step A→B→C structure)
- Check alert rules file has Grafana 12+ format
- Use export from Grafana 12+ or convert manually

**Grafana dashboard errors**
- Dashboard version incompatibility
- Missing Grafana plugins
- Ad-hoc filter variables not supported
- This is a dashboard issue, not a bundle deployer issue

### Debug Mode

Add verbose logging:

```typescript
// In any file
console.log(`DEBUG: ${JSON.stringify(value, null, 2)}`);
```

Check TypeScript compilation:
```bash
deno check src/main.ts
```

### Getting Help

1. **Check validation errors first** - They're usually descriptive
2. **Run without deployment** - Validate only to isolate issues
3. **Check environment variables** - Most auth errors are env vars
4. **Look at Hydrolix/Grafana UI** - See what actually got created
5. **Check Docker logs** - `docker logs <container_id>`

## Development

### Running Type Checks

```bash
deno check src/main.ts
```

### Formatting Code

```bash
deno fmt src/
```

### Adding a Validation Module

1. Create `src/validation/your_check.ts`:
```typescript
import type { Bundle } from "../types/bundle.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  // Your validation logic
  if (invalid) {
    throw new Error("Descriptive error message");
  }
}
```

2. Import in `src/main.ts`:
```typescript
import * as your_check from "./validation/your_check.ts";
```

3. Call in validation pipeline:
```typescript
await your_check.run(base, bundle);
```

### Adding a CLI Flag

1. Parse in `main.ts`:
```typescript
const YOUR_FLAG = args.includes("--your-flag");
```

2. Use in logic:
```typescript
if (YOUR_FLAG) {
  // Special behavior
}
```

### Adding Template Variable Support

1. Add replacement in `deploy.ts`:
```typescript
dashboard = dashboard.replace(/__YOUR_VAR__/g, yourValue);
```

2. Document in bundle template
3. Use in dashboard/alert files

## Architecture Decisions

### Why TypeScript/Deno?

**vs Rust:**
- ✅ 36% less code (2,250 vs 3,500 lines)
- ✅ Simpler error handling (try/catch vs Result<T,E>)
- ✅ Native JSON support (no serde)
- ✅ Faster iteration (no compilation)
- ✅ Easier for JS/TS developers

**vs Node.js:**
- ✅ Secure by default (explicit permissions)
- ✅ Built-in TypeScript support
- ✅ Modern standard library
- ✅ Single executable

### Why Individual Validation Modules?

- **Single Responsibility** - Each module checks one thing
- **Testable** - Can unit test in isolation
- **Composable** - Easy to add/remove validators
- **Clear Errors** - Obvious which check failed

### Why Two Deploy Files?

**deploy.ts** (Full deployment):
- Creates tables, inserts data
- Slow but complete
- For thorough testing

**deploy_only_dashboard.ts** (Fast deployment):
- Just dashboard and alerts
- Quick iteration on visualizations
- Assumes data already exists

### Why Warnings vs Errors?

**Errors (stop execution):**
- Bundle structure invalid
- Required files missing
- Critical validation failures

**Warnings (continue):**
- Dictionary creation failed (might be pre-loaded)
- Function already exists (idempotent)
- Sample data insertion failed (dashboard still works)

**Philosophy:** Fail fast on bundle issues, be tolerant of infrastructure issues.

## Performance

### Typical Timing

**Validation only:** ~2 seconds
**Dashboard-only deployment:** ~30 seconds
**Full deployment (6 transforms):** ~5-8 minutes

**Breakdown:**
- Function/dictionary creation: ~10 seconds
- Table creation + wait: ~30 seconds per table
- Transform addition: ~5 seconds per transform
- Sample data insertion: ~30 seconds per transform
- Summary table creation: ~10 seconds
- Dashboard creation: ~5 seconds
- Alert rules creation: ~5 seconds
- Headless browser testing: ~120 seconds

### Optimization Tips

- Use `--local-dashboard-only` during development
- Remove sample data insertion for faster iteration
- Reduce `TABLE_READY_DELAY_SECS` if your cluster is fast
- Skip headless testing during development

## Migration from Rust Version

This TypeScript version is **feature-complete** with the Rust version and includes additional features:

**New features:**
- ✅ Alert rules support (Grafana 12+)
- ✅ YAML/Regexp dictionary support
- ✅ JSON dictionary definitions
- ✅ Individual function creation (more robust)
- ✅ Better error messages
- ✅ Smart dependency handling

**Maintained features:**
- ✅ All validation checks
- ✅ Template variable replacement
- ✅ Headless browser testing
- ✅ Docker container management
- ✅ Production mode
- ✅ Summary table support

## Contributing

### Code Style

- Use explicit return types for functions
- Prefer `const` over `let`
- Use optional chaining (`?.`) for nested properties
- Add descriptive error messages with context
- Include comments for non-obvious logic

### Testing

Before committing:
```bash
# Type check
deno check src/main.ts

# Format
deno fmt src/

# Test with a real bundle
deno run --allow-all src/main.ts --local test_bundle
```

## License

[Your License Here]

## Support

For issues or questions:
- Check this README
- Review `ALERT_RULES_IMPLEMENTATION.md` for alert-specific details
- Check Hydrolix docs: https://docs.hydrolix.io
- Check Grafana docs: https://grafana.com/docs