---
name: Bundle Validation Architecture
description: How the Rust validator and Python configurator work together; substitution token rules, sample_data freshness check location, and recurring pitfalls
type: project
---

The bundle system has two validation layers:

**Rust validator** (`src/validate/`):
- `template_variable_consistency.rs`: Scans all dashboard JSON files (primary + `other_dashboards`) for `__UPPER_CASE__` tokens. Only tokens declared in `bundle.json` (tables, summary tables) plus `__PROJECT_NAME__`, `__DATASOURCE__`, `__DASHBOARD_UUID__` are allowed. Standard Grafana built-ins (`__time*`, `__from*`, `__to*`, `__dashboard`, `__user`, `__interval*`) are also exempted. Any other `__TOKEN__` is a hard error.
- `dashboard_is_valid.rs`: Verifies `__DASHBOARD_UUID__`, `__DATASOURCE__`, `__PROJECT_NAME__`, and all table dashboard_vars are present in every dashboard.
- `sample_data_freshness.rs`: Reads `settings.sample_data` from INSIDE each `transform.json` (NOT the standalone `sample_data.json` sidecar file). Staleness threshold is 183 days. Only checks transforms listed in `bundle.json > tables[].transforms[]`. Only fires on transforms whose primary column is `type: "epoch"` (datetime-type primary columns are skipped by the `find_map` filter).

**Python configurator** (`scripts/configurator/dashboard_fixer.py`):
- `_fix_template_variables`: Handles constant-type template vars. Passes through any variable it doesn't recognize — it does NOT rewrite `raw_logs` or `cdn_panel` type constants unless they match known patterns (`raw_table`, VAR_* refs, `timestamp`).
- `_fix_uid`: Sets every dashboard's `uid` field to `__DASHBOARD_UUID__`.
- `DASHBOARD_UUID_TEMPLATE` constant (`constants.py:98`) = `"__DASHBOARD_UUID__"`.

**Key pitfall (seen in cdn-insights v1.1.0 fix commit, confirmed unresolved as of 2026-04-13):**
- The standalone `transformations/default/sample_data.json` is NOT what `sample_data_freshness.rs` reads. It reads `settings.sample_data` embedded in `transform.json`. Updating only the sidecar leaves the validator-facing stale timestamp untouched.
- `transformations/default/transform.json` at `settings.sample_data.timestamp` is `1743580800` (2025-04-02). As of 2026-04-13 this is ~376 days old — well past the 183-day threshold. The validator WILL emit a staleness warning for this transform.
- `raw_logs` and `cdn_panel` are Grafana constant template vars in `CDN Global View.json` used as dashboard cross-links (holding a UID). The validator fix in 7d6f3f3 pointed both to `__DASHBOARD_UUID__`. This silences the validator error. However, at deploy time `__DASHBOARD_UUID__` resolves to the UID of whichever dashboard is currently being provisioned — not stable per-dashboard identifiers. Both `raw_logs` and `cdn_panel` will end up pointing to the same UID (the CDN Global View's own UID at provisioning time), collapsing two distinct cross-links into one.

**Why:** bundle.json `other_dashboards` lists both `CDN Global View.json` and `Raw Logs.json`, but there is only one `__DASHBOARD_UUID__` token — it is replaced with the UID of whichever dashboard is currently being provisioned, not a stable per-dashboard identifier.

**`__SHARED_PROJECT__` token:** Used in `sql_transform` strings inside transform.json files — NOT scanned by `template_variable_consistency.rs` (which only reads dashboard JSON files). Safe.
