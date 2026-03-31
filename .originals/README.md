# .originals/ Directory

This directory stores the **raw, pre-pipeline versions** of bundle assets. It is the source of truth for what assets looked like before the CI pipeline formatted and configured them.

## Why this exists

The CI pipeline (Stage 2 — `configure_bundle.py`) modifies assets in-place: it injects template variables, strips `__inputs` from transforms, wraps dashboards, and renames directories. These transformations are **not reversible** — once assets are configured, re-running the pipeline on them produces corrupted output (double-injected variables, missing data, etc.).

`.originals/` solves this by preserving a clean copy of the raw assets from the first pipeline run. When the pipeline needs to re-run (e.g., after a contributor updates raw assets), it restores from `.originals/` first, ensuring a clean starting point every time.

## How it works

- **First pipeline run:** The pipeline automatically backs up raw assets here before formatting. No manual action needed.
- **Re-runs:** The pipeline restores from `.originals/`, then re-formats from scratch, producing identical results.
- **Editing originals:** If you edit files directly in `.originals/`, the pipeline detects this and runs a full re-format (Track 1) from the updated originals.

## Directory structure

`.originals/` mirrors the bundle path structure in the main repository:

```
.originals/
├── aws/
│   └── cdn-insights/
│       ├── dashboards/
│       ├── transformations/
│       └── summaries/
└── trafficpeak/
    └── default_shared/
        └── 1.0.6/
            ├── dashboards/
            └── transformations/
```

Both versioned paths (e.g., `trafficpeak/default_shared/1.0.6/`) and legacy unversioned paths (e.g., `aws/cdn-insights/`) are supported.

**Excluded from backups:** `bundle.json` and `bundle-config.json` are not stored here — `bundle.json` is auto-generated, and `bundle-config.json` is configuration metadata, not a raw asset.

## For external contributors

You do not need to interact with this directory. Place your raw assets in the normal bundle directory (`aws/` or `trafficpeak/`) and the pipeline handles everything automatically. If you need to update assets after the initial pipeline run, simply push new raw files to the normal directory — the pipeline will detect them and update `.originals/` for you.

## For internal team

- To trigger a full re-pipeline for a bundle, edit files in `.originals/` and push. The pipeline will restore those originals and re-run the full format + validate flow.
- To inspect what the original raw assets looked like before formatting, check the corresponding path in `.originals/`.
- Legacy bundles that predate this system will not have an `.originals/` entry. They run validation-only (Track 2) by default and work as-is.
