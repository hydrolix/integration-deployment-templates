# CLAUDE.md — Integration Deployment Templates

## Running tests

```bash
python3 -m pytest tests/ -v
```

Tests use pytest with `tmp_path` fixtures. Python path includes both `.` and `scripts/` (see `pyproject.toml`).

## Project structure

- `scripts/configurator/` — Pipeline phases (discovery, transform org, SQL analysis, dashboard fixer, bundle JSON builder)
- `scripts/utils/` — Shared utilities (file I/O, JSON helpers)
- `scripts/detect_track.py` — Two-track CI: classifies bundles as raw/configured, routes to full or validate-only pipeline
- `scripts/originals_manager.py` — Manages `.originals/` backup/restore for the full pipeline track
- `tests/` — Pytest tests
- `trafficpeak/`, `aws/` — Bundle directories (source/bundle/version structure)
- `.originals/` — Backup of raw assets before configuration (not committed)

## Key conventions

- Configurator modules use **relative imports** (e.g., `from .constants import ...`) and expect `utils` on the Python path
- Transform JSON lives at `transformations/transform.json` within each bundle; `settings.output_columns` contains column definitions with `datatype.primary` marking the timestamp column
- Template variables: `__PROJECT_NAME__`, `__TABLE_NAME__`, `__SUMMARY_TABLE_NAME_N__`, `__DATASOURCE__`, `__DASHBOARD_UUID__`
- Primary vs other dashboards: primary uses `__SUMMARY_TABLE_NAME_N__` (no prefix), others use `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_N__`

## Gotchas

- **VAR_* references in non-SQL contexts are intentional.** Variables like `VAR_CDN_PANEL` in link URLs and panel references work fine at runtime — only self-referencing VAR_* constants in SQL contexts cause ClickHouse errors. Don't fix VAR_* patterns unless they're self-referencing constants (variable named X with query `${VAR_X}`).
- **`scripts/` has no `__init__.py`** but `scripts/configurator/` and `scripts/utils/` do. Top-level scripts like `detect_track.py` are imported as `scripts.detect_track`, while configurator modules use relative imports within their package.
- **BundleConfig requires `table_name` and `data_category`** as positional args — don't forget these when constructing in tests.
- **Dry-run mode reads from `original_path`**, not `final_path`, on TransformInfo objects.
