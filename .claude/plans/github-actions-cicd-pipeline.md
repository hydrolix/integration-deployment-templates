# GitHub Actions CI/CD Pipeline Plan (MVP)

## Context

External teams submit raw integration assets via PRs to this repo. Currently, the Hydrolix team manually pulls branches, runs scripts locally, and pushes results. This plan automates the formatting, configuration, and structural validation steps with GitHub Actions, while keeping the manual `--local` validation and Grafana dashboard review as a human gate before merge.

**Coworker's `push-to-cac` workflow** (pasted in chat) is the starting reference for the export piece — uses GitHub App auth via `actions/create-github-app-token@v2`.

## Decisions Made

| Decision | Choice |
|----------|--------|
| Config input | `bundle-config.json` file in bundle dir |
| PR pipeline | Combined workflow first, break out later |
| What push-to-cac copies | Portables (YAML CaC format) |
| Auth for cross-repo | GitHub App token (coworker's pattern) |
| Target repo | `hydrolix/cac-tools-test` |
| Validation in CI | Structural only (no `--local`) |
| `--local` + Grafana review | Manual step before merge (human gate) |

---

## MVP Flow (End-to-End)

```
1. External team opens PR with raw assets + bundle-config.json
   │
   ▼
2. bundle-ci.yml triggers (auto)
   ├── detect-changes → list changed bundle dirs with bundle-config.json
   ├── format-bundles → run_pipeline.py (bundle_to_yaml + configure_bundle), commit back to PR
   └── validate-bundles → Rust validator (structural, no --local)
   │
   ▼ (CI green check on PR)
   │
3. Hydrolix team pulls the PR branch (manual)
   ├── Run validator with --local against test cluster
   ├── Verify Hydrolix tables: transforms validate, data in table
   └── Verify Grafana dashboard: no console errors, panels load, data renders
   │
   ▼
4. Manual QA testing (manual)
   │
   ▼
5. PR merged to main
   │
   ▼
6. push-to-cac.yml triggers (auto)
   ├── Copy portables/<bundle>/ to cac-tools-test
   ├── Create PR in cac-tools-test
   └── Clean up portables/ from integration-deployment-templates

bundle-validator.yml (simplified)
  └── Push to main only (safety net for direct pushes)
```

### Why structural-only validation in CI

Running `--local` in CI would require:
- A dedicated Hydrolix cluster instance for CI
- Unique project/table names per PR to avoid collisions between concurrent PRs
- Automated table cleanup after validation
- No good way to automate the Grafana visual review ("do these dashboards look right?")

These are solvable but add significant infrastructure complexity. For MVP, the CI catches schema/format issues automatically, and the human review step catches integration issues. This can be revisited once the basic pipeline is proven.

---

## File Changes

### New Files
1. **`.github/workflows/bundle-ci.yml`** — Combined formatter + structural validator for PRs
2. **`.github/workflows/push-to-cac.yml`** — Merge-triggered export to cac-tools-test
3. **Example `bundle-config.json`** files for existing bundles (documentation)

### Modified Files
4. **`.github/workflows/bundle-validator.yml`** — Remove `pull_request` trigger (handled by bundle-ci now), keep `push: [main]` only

---

## Workflow 1: `bundle-ci.yml` (NEW)

**Trigger:** `pull_request` targeting `main`, paths `aws/**` or `trafficpeak/**`

### Job 1: `detect-changes`
- Checkout with `fetch-depth: 0`
- `git diff --name-only origin/main...HEAD` to find changed files
- Extract unique bundle dirs (`aws/<name>` or `trafficpeak/<name>`)
- Filter to dirs that contain a `bundle-config.json` (warn on missing)
- Output: JSON array of bundle dirs, boolean `has_bundles`

### Job 2: `format-bundles` (needs: detect-changes)
- Checkout PR branch with `ref: ${{ github.head_ref }}`
- Setup Python 3.11, `pip install pyyaml`
- Loop through each detected bundle:
  - Read `bundle-config.json` with `jq` to extract `table_name` and `data_category`
  - Run `python scripts/run_pipeline.py --bundle-dir <dir> --table-name <name> --data-category <cat> --config <dir>/bundle-config.json --skip-validate --verbose`
  - This runs both `bundle_to_yaml.py` (stage 1) and `configure_bundle.py` (stage 2)
- Commit and push all changes:
  - `git config` as `github-actions[bot]`
  - `git add aws/ trafficpeak/ portables/` (explicit paths, not `-A`)
  - Skip if no changes: `git diff --staged --quiet`
  - Commit message: `[skip ci] Auto-format bundles via bundle-ci`
  - `git push`
- **Commit loop prevention:** `[skip ci]` in message + default `GITHUB_TOKEN` (pushes with default token don't trigger `pull_request` events)

### Job 3: `validate-bundles` (needs: format-bundles)
- Runs in `rust:latest` container
- Checkout PR branch (gets formatter's committed changes)
- Install Rust toolchain, cache cargo deps (same pattern as existing workflow)
- `cargo build --release --bin bundle-validator`
- `./target/release/bundle-validator` with secrets from `bundle-validator-env`
- **No `--local` flag** — structural validation only

### Key design notes:
- Uses `run_pipeline.py --skip-validate` to call both Python scripts in one shot, keeping pipeline logic centralized
- `table_name` and `data_category` are parsed from `bundle-config.json` via `jq` in the shell step (workaround for `run_pipeline.py`'s `_require_stage2_args` check which requires these as CLI args even when `--config` is passed)
- Single sequential loop (not matrix) for simplicity — multi-bundle PRs are rare, parallel matrix can be added later
- The validate job uses the existing Rust container + environment pattern from `bundle-validator.yml`

---

## Manual Review Step (Between CI and Merge)

After CI passes (green check), the Hydrolix team:

1. **Pull the PR branch locally** — all formatted/configured assets are already committed by the format job
2. **Run `--local` validation:**
   ```bash
   cargo run -- --local <bundle_name>
   ```
3. **Check Hydrolix cluster** — verify tables created, transforms validate, data present
4. **Check Grafana dashboard** — spin up localhost Grafana (or point at an instance), verify panels load, no console errors, data renders correctly
5. **QA sign-off** — any additional manual testing
6. **Merge the PR** — triggers push-to-cac export

This preserves the current review workflow exactly — CI just handles the tedious format/configure steps automatically.

---

## Workflow 2: `push-to-cac.yml` (NEW — adapted from coworker's)

**Trigger:** `pull_request: types: [closed]` on `main` (where `merged == true`) + `workflow_dispatch` for testing

### Key changes from coworker's version:
1. **Copy portables instead of raw bundles** — `cp -r integrations-deployment-templates/portables/<bundle_name> cac-tools-test/data/bundles/<bundle_name>`
2. **Portables cleanup step** — After creating the PR in cac-tools-test, commit removal of `portables/` back to main in integration-deployment-templates
3. **Keep everything else** — GitHub App auth, detect-changes logic, PR creation with metadata

### Steps:
1. Create GitHub App token (`INTEGRATIONS_APP_ID` + `INTEGRATIONS_APP_PRIVATE_KEY`)
2. Checkout integration-deployment-templates (main, with `fetch-depth: 2` for diff)
3. Checkout `hydrolix/cac-tools-test` with app token
4. Detect portable bundles to export — `git diff --name-only HEAD~1 HEAD` to find files changed by the merge, filter to `portables/` paths, extract bundle names
5. For each portable bundle, copy `portables/<bundle_name>/` → `cac-tools-test/data/bundles/<bundle_name>/`
6. Create branch, commit, push, `gh pr create` in cac-tools-test (same PR format as coworker's: source PR #, commit SHA, triggering user)
7. **Cleanup**: back in integration-deployment-templates, remove the exported `portables/` dirs, commit to main with `[skip ci]`

### Portables directory mapping:
- Source: `portables/<bundle_name>/<version>/` (e.g., `portables/bot-insights-cdn/1.0.0/`)
- Target: `cac-tools-test/data/bundles/<bundle_name>/` — confirm exact target path with coworker

---

## Workflow 3: `bundle-validator.yml` (MODIFIED)

Simplify to main-only safety net:

```yaml
name: bundle-validator
on:
  push:
    branches: [main]
```

Remove `pull_request` trigger since `bundle-ci.yml` now handles PR validation. Keep all existing steps unchanged.

---

## `bundle-config.json` Convention

PR submitters include this file in their bundle directory:

```json
{
  "table_name": "bot_detection",
  "data_category": "security"
}
```

**Required fields:** `table_name`, `data_category`

**Optional fields** (override auto-inference): `source_name`, `bundle_name`, `channel_type`, `method`, `description`, `version`, `beta`

The `configure_bundle.py --config` flag already consumes this format via `_build_config_from_dict()`.

---

## Implementation Order

1. **Create `bundle-ci.yml`** — the core PR workflow (format + structural validate)
2. **Modify `bundle-validator.yml`** — remove PR trigger
3. **Create `push-to-cac.yml`** — adapt coworker's workflow for portables
4. **Add example `bundle-config.json`** to an existing bundle for testing
5. **Update plan docs** — refresh `configure-bundle-pipeline.md`

---

## Required Secrets / Vars

| Secret/Var | Workflow | Notes |
|-----------|----------|-------|
| `BUNDLE_TESTING_CLUSTER` | bundle-ci, bundle-validator | Already configured in `bundle-validator-env` |
| `BUNDLE_TESTING_USERNAME` | bundle-ci, bundle-validator | Already configured |
| `BUNDLE_TESTING_PASSWORD` | bundle-ci, bundle-validator | Already configured |
| `INTEGRATIONS_APP_ID` | push-to-cac | GitHub App ID (var, not secret) |
| `INTEGRATIONS_APP_PRIVATE_KEY` | push-to-cac | GitHub App private key |

---

## Open Items to Confirm

- [ ] Exact target path in `cac-tools-test` for portables (is it `data/bundles/<name>/` or root?)
- [ ] Whether portables cleanup commit to main needs approval or can auto-commit
- [ ] Python dependency: confirm `pyyaml` is the only external dep needed in CI
- [x] `run_pipeline.py` already passes `--config` through to `configure_bundle.py` (line 139, 329)
- [x] Validation depth: MVP uses structural-only in CI, manual `--local` + Grafana review before merge

---

## Exploration Findings (Technical Notes)

### `run_pipeline.py` CLI gap
`run_pipeline.py` requires `--table-name` and `--data-category` as its own CLI args even when `--config` is passed (the `_require_stage2_args` check on line 197-207 runs before `configure_bundle.py` ever sees the config file). The CI workflow works around this by parsing `bundle-config.json` with `jq` and passing values as both CLI args and `--config`.

### Coworker's workflow reference
Full workflow YAML recovered from planning session. Key patterns to reuse:
- `actions/create-github-app-token@v2` with `owner: hydrolix`
- Dual checkout pattern (both repos side by side)
- `gh pr create` with `GH_TOKEN` from app token
- Branch naming: `push-to-cac-${PR_NUMBER}-$(date +%s)`

---

## Verification

1. **Test bundle-ci:** Open a PR adding/modifying a bundle dir with a `bundle-config.json` → CI should auto-format, commit, and validate (structural)
2. **Test manual review:** Pull the CI-formatted branch locally → `cargo run -- --local <bundle>` should work against the formatted assets
3. **Test push-to-cac:** Merge that PR → workflow should create a PR in cac-tools-test with portables
4. **Test no-op:** Open a PR that only changes non-bundle files (e.g., README) → workflows should skip gracefully
5. **Test missing config:** Open a PR with bundle changes but no `bundle-config.json` → formatter should warn and skip that bundle

---

## Future Iterations (Post-MVP)

- **Automated `--local` validation:** Namespaced ephemeral tables per PR, automated cleanup, dedicated CI cluster
- **Grafana screenshot comparison:** Headless browser captures for visual regression detection
- **Matrix builds:** Parallel per-bundle formatting for multi-bundle PRs
- **PR comment bot:** Post validation results as PR comments with links to dashboard previews
