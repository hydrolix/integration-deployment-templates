# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Repo Is

Hydrolix integration deployment templates — a two-track CI/CD system for validating, configuring, and deploying integration bundles. Bundles live in `aws/` and `trafficpeak/` directories and contain transforms, dashboards, summaries, functions, and dictionaries that get validated by a Rust binary and deployed to Hydrolix clusters.

## Build & Test Commands

### Rust Validator
```bash
cargo build                          # build debug binary
cargo run                            # validate all bundles (basic, no headless browser)
cargo run -- cloudfront              # validate only bundles matching "cloudfront"
cargo run -- --local                 # full testing with Grafana container + headless browser
cargo run -- --output                # dump detailed JSON validation output
cargo fmt                            # format
cargo clippy                         # lint (warns on unwrap/expect/panic/todo)
```

### Python Pipeline
```bash
python3 scripts/run_pipeline.py --bundle-dir <dir> --config <dir>/bundle-config.json --track full --verbose
python3 scripts/configure_bundle.py --bundle-dir <dir> --table-name <name> --data-category <cat>
python3 scripts/sync_cluster_deps.py --bundle-dir <dir> --dry-run
python3 scripts/bundle_to_yaml.py --source <dir>
```

### Tests
```bash
python3 -m pytest tests/                              # all Python tests
python3 -m pytest tests/test_timestamp_freshness.py   # single test file
python3 -m pytest tests/test_detect_track.py -v       # verbose single file
cargo test                                            # Rust tests
```

### Make Targets
```bash
make quick              # basic validation (no headless)
make full               # marketplace + headless browser
make coding-standards   # cargo fmt + clippy + ? operator ban
make audit              # security audit
make clean              # prune docker and cargo
make git-actions-locally # run CI locally via act
```

## Architecture

### Two-Track CI Pipeline (`bundle-ci.yml`)

**Track 1 — Full Pipeline** (raw bundles with `bundle-config.json`):
1. **Stage 1** (`bundle_to_yaml.py`): Generate portable CaC YAML in `portables/`
2. **Stage 2** (`configure_bundle.py`): Normalize raw assets — organize transforms, inject template variables, generate `bundle.json`, fix dashboards/summaries
3. **Stage 3** (`sync_cluster_deps.py`): Sync missing functions/dictionaries to Hydrolix cluster
4. **Stage 4**: Rust validator

**Track 2 — Validate Only** (already-configured bundles): Runs only the Rust validator.

Track detection (`scripts/detect_track.py`): if a bundle has `bundle-config.json` and non-generated files changed, it routes to Track 1. Changes to `.originals/` always trigger Track 1.

### Key Components

**Rust Validator** (`src/`): 18 validation modules checking bundle.json schema, transform validity, sample data freshness (183-day threshold), dashboard template variables, dependency resolution, datasource UIDs, duplicate tokens across bundles, and optional Grafana headless browser rendering. Clippy warns on unwrap/expect/panic/todo (via `-W`), and the `?` operator is banned in `src/` via grep check in `make coding-standards`.

**Configurator Engine** (`scripts/configurator/`): 8-phase Python pipeline that transforms raw vendor exports into configured bundles. Phases: discovery → transform organization → SQL analysis → bundle.json build → summary fixing → dashboard fixing → bundle.json update → reporting.

**Deployment** (`src/deploy/`): Creates temporary Hydrolix projects, deploys transforms + dashboards, inserts sample data for validation, then cleans up. After each ingest, `verify_rows_ingested_if_present` (in `default.rs`, backed by pure helpers in `verify.rs`) polls `SELECT count()` on the test table for up to 60s. On a zero-count timeout it runs four diagnostics — primary-timestamp staleness, `system.parts`, `system.ingest_errors`, missing sample_data fields — and fails the deploy with the findings embedded in the error. HTTP 2xx from `/ingest/event` alone is not sufficient to consider a bundle valid.

### The `.originals/` Directory

Stores backup copies of raw assets before Stage 2 modifies them. When a contributor pushes raw assets, CI backs them up here, then runs the full pipeline on the working copy. Restoring from `.originals/` triggers a full pipeline re-run.

## Bundle Conventions

- Bundles live in `aws/{name}/` or `trafficpeak/{name}/` (flat, no version subdirs)
- `bundle.json` is the manifest; `bundle-config.json` triggers Track 1 auto-generation
- Template variables: `__PROJECT_NAME__`, `__TABLE_NAME__`, `__DATASOURCE__`, `__DASHBOARD_UUID__`, `__SUMMARY_TABLE_NAME_N__`
- Table names: only letters, digits, underscores (no dashes)
- `aws/` bundles use `commons_` SQL prefix; `trafficpeak/` uses `akamai_` prefix
- Primary dashboard summary vars omit `__PROJECT_NAME__`; other dashboards include it
- Portables output goes to `portables/{source}/{bundle_name}/1.0.0/`
- `cargo run` arg is a bundle name substring filter, not a file path

## Validator Arg Note

The Rust validator's positional argument filters bundles by `bundle.name` field (from bundle.json), not by directory path. `cargo run` with no args validates all bundles. `cargo run -- bot_insights` validates only bundles whose name contains "bot_insights".

## Environment Variables (CI/cluster testing)

- `BUNDLE_TESTING_CLUSTER` — Hydrolix cluster endpoint
- `BUNDLE_TESTING_USERNAME` / `BUNDLE_TESTING_PASSWORD` — Auth credentials
- `STRICT_PLUGIN_VALIDATION` — Enable strict Grafana plugin checking

## Pre-commit Hooks

Configured via `.pre-commit-config.yaml`: rustfmt, clippy, cargo check, JSON pretty-formatting, YAML/TOML validation, trailing whitespace, end-of-file fixer, merge conflict detection. JSON files get auto-sorted by the pretty-format-json hook — expect commits to fail on first attempt then pass after re-staging.
