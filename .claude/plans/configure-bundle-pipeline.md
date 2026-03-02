# Bundle Configuration Pipeline Plan

## Pipeline Overview (Long-Term)

Upload raw assets → (1) Generate CAC bundle from raw assets → (2) **configure_bundle.py** → (3) Validate with Rust validator → (4) Deploy CAC to cac-tools repo as PR.

## Current Status

### Phase 2: configure_bundle.py - IMPLEMENTED
- `scripts/configure_bundle.py` - CLI entry point
- `scripts/configurator/` - 7-phase module package
- `scripts/utils/file_utils.py` - shared utilities

**Tested against:** `trafficpeak/bot-insights-cdn` (raw multi-provider bundle)
**Rust validator:** Passes individually. Global duplicate check for `ui.source.full_title` requires manual override.

### Phase 1: CAC Bundle Generation - NOT STARTED
- Converts raw vendor exports into the initial bundle structure
- Would feed into configure_bundle.py

### Phase 3: Rust Validator Integration - NOT STARTED
- After configure_bundle.py runs, automatically invoke `cargo run -- <bundle_name>`
- Parse validator output for errors vs warnings
- Could be a `--validate` flag on configure_bundle.py or a separate wrapper script

### Phase 4: CAC Deploy as PR - NOT STARTED
- Push configured bundle to cac-tools repo
- Create PR with bundle contents
- GitHub Actions integration

---

## configure_bundle.py - Implementation Details

### File Structure
```
scripts/
├── configure_bundle.py              # Main CLI entry point
├── configurator/                    # Package
│   ├── __init__.py
│   ├── config.py                    # BundleConfig + BundleState dataclasses
│   ├── constants.py                 # Prefix maps, valid enums, regex patterns
│   ├── discovery.py                 # Phase 1: scan bundle dir, catalog assets
│   ├── transform_organizer.py       # Phase 2a-2d: normalize, clean, extract sample data
│   ├── sql_analyzer.py              # Phase 2e: parse SQL, fix prefixes, collect deps
│   ├── bundle_json_builder.py       # Phase 3: generate bundle.json
│   ├── summary_fixer.py             # Phase 4: fix hardcoded tables in SQL
│   ├── dashboard_fixer.py           # Phase 5: wrapper, UIDs, template vars, datasources
│   ├── bundle_json_updater.py       # Phase 6: add dashboard/summary paths
│   └── report.py                    # Phase 7: structured output report
└── utils/
    ├── __init__.py
    └── file_utils.py                # read_json, write_json, write_file, etc.
```

### CLI Interface
```bash
python3 scripts/configure_bundle.py \
  --bundle-dir trafficpeak/bot-insights-cdn \
  --table-name bot_detection \
  --data-category security
```

#### Required args
| Arg | Description |
|-----|-------------|
| `--bundle-dir` | Path to bundle directory (relative to repo root or absolute) |
| `--table-name` | Table name - only letters, digits, underscores (e.g., `bot_detection`) |
| `--data-category` | `security` \| `cdn` \| `video` |

#### Optional args (auto-inferred)
| Arg | Default / Inference |
|-----|---------------------|
| `--source-name` | From path: 1st component (e.g., `trafficpeak`) |
| `--bundle-name` | From path: 2nd component (e.g., `bot-insights-cdn`) |
| `--channel-type` | `aws/*` → `AWS`, `trafficpeak/*` → `3rdParty` |
| `--maintainer` | `Hydrolix Team <team@hydrolix.io>` |
| `--description` | Auto: `"{Source} {Bundle} Integration"` |
| `--version` | `1.0.0` |
| `--method` | Auto from filenames (firehose/kinesis/http_streaming/multi_stream) |
| `--primary-dashboard` | Auto: `home.json` > `default.json` > `overview.json` > only-file |
| `--beta` / `--no-beta` | Default `true` |
| `--verbose` | Detailed progress to stderr |
| `--dry-run` | Show what would change without writing |
| `--config` | JSON config file (alternative to CLI args for CI/CD) |

#### Exit codes
- `0` = success
- `1` = validation/processing error
- `2` = missing required input

#### Output
JSON report to stdout with: phases completed, files modified, warnings, errors.

### Phase Details

#### Phase 1: Discovery (`configurator/discovery.py`)
- Scan bundle dir for `transformations/` or `transforms/`, `dashboards/` or `grafana/`, `summaries/`
- Catalog each transform file, dashboard JSON, summary SQL
- Detect multi-provider transforms (multiple JSON files at root)
- Error if no transforms found

#### Phase 2a-2d: Transform Organization (`configurator/transform_organizer.py`)
- **2a**: Rename `transforms/` → `transformations/`, `grafana/` → `dashboards/`
- **2b**: Organize transforms: single (no-op), rename to `transform.json`, or create provider subdirs for multi-provider
- **2c**: Strip metadata fields from transforms: `uuid`, `created`, `modified`, `url`, `table`
- **2d**: Extract `sample_data` from each transform → write `sample_data.json`. Normalize arrays to single object `[0]`. Error if missing.

#### Phase 2e: SQL Analysis (`configurator/sql_analyzer.py`)
- Parse `settings.sql_transform` in each transform
- Fix prefixes: `aws/` bundles → `commons_`, `trafficpeak/` → `akamai_`
- Replace `reference_`/`commons_`/`akamai_` with correct prefix
- Collect unique function/dictionary base names for `shared_functions`/`shared_dictionaries`
- Skip transforms without `sql_transform`

#### Phase 3: Build bundle.json (`configurator/bundle_json_builder.py`)
- Detect method: multi transforms → `multi_stream`; single → check filename for `firehose`/`kinesis`, else `http_streaming`
- Build `tables[].transforms[]` with correct relative paths
- Populate `dependencies.hydrolix.shared_functions` and `shared_dictionaries`
- If `bundle.json` exists: merge, preserving `method_overrides`, `alert_rules`, manual deps

#### Phase 4: Fix Summaries (`configurator/summary_fixer.py`)
- Replace hardcoded `FROM schema.table` → `__PROJECT_NAME__.__TABLE_NAME__`
- Skip already-templated references (idempotent)
- Assign `__SUMMARY_TABLE_NAME_N__` dashboard vars

#### Phase 5: Fix Dashboards (`configurator/dashboard_fixer.py`)
- **5a**: Wrap in `{"dashboard": {...}}` if missing; add `__elements` with datasource model; remove `__inputs`
- **5b**: Set `uid` → `__DASHBOARD_UUID__`
- **5c**: Fix template variables:
  - **Primary dashboard**: summary vars use `__SUMMARY_TABLE_NAME_N__` (NO `__PROJECT_NAME__` prefix)
  - **Other dashboards**: summary vars use `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_N__` (WITH prefix)
  - Replace `${VAR_*}` references using `__inputs` mapping
  - Add hidden `raw_table` variable
- **5d**: Replace all datasource UIDs → `__DATASOURCE__` (preserve Grafana special UIDs)

#### Phase 6: Update bundle.json (`configurator/bundle_json_updater.py`)
- Set `dashboard.path` to primary dashboard
- Set `other_dashboards[]` paths
- Add `summary_tables[]` from Phase 4

#### Phase 7: Report (`configurator/report.py`)
- Generate structured JSON report to stdout

### Key Edge Cases
| Case | Handling |
|------|----------|
| Multi-provider transforms (5+ files) | Create subdirs, per-provider method detection |
| Multiple dashboards, unclear primary | Auto-detect by name (`home`/`default`/`overview`), else require `--primary-dashboard` |
| Sample data is array `[{},{}]` | Take `[0]`, normalize to single object |
| Transform missing `sample_data` | Fatal error |
| No `sql_transform` field | Skip SQL analysis for that transform |
| No summaries | Skip Phase 4, set `summary_tables: []` |
| Already-configured bundle (re-run) | Idempotent: skip renames if target exists, regex skips template vars |
| Existing bundle.json with manual overrides | Merge: preserve `method_overrides`, `alert_rules`, manual deps |
| Table name with dashes | Rejected at CLI validation (only letters, digits, underscores) |

### Critical Reference Files
| File | Role |
|------|------|
| `.claude/skills/configure-bundle/skill.md` | Authoritative spec for all phases |
| `src/models/bundle.rs` | bundle.json schema constraints |
| `src/validate/*.rs` | What the Rust validator checks |
| `src/deploy/default.rs` | Why primary vs other dashboards differ |

---

## Remaining Work / Future Enhancements

### Near-term
- [ ] Test with more bundle types (single-transform, no summaries, multiple dashboards)
- [ ] Add `--validate` flag to auto-run Rust validator after configuration
- [ ] Config file test: provide JSON config instead of CLI args
- [ ] Handle `ui.source.full_title` uniqueness (warn if collision detected)

### Medium-term
- [ ] Phase 1 (CAC generation) script
- [ ] CI/CD GitHub Actions workflow that runs configure + validate
- [ ] Phase 4 (deploy to cac-tools as PR)

### Long-term
- [ ] UI orchestration layer integration
- [ ] Batch processing of multiple bundles
