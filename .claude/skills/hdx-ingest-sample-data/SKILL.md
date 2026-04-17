---
name: hdx-ingest-sample-data
description: Ingest N rows of realistic test data into any Hydrolix table. Automatically finds the transform's sample_data from bundle files, sets timestamps distributed across a given date range, and handles all authentication variants (JWT-only or JWT + table ingest token). Use whenever the user wants to populate a table with test data, seed data for dashboard testing, ingest sample events, or run ingestion tests on any cluster. Trigger on phrases like "ingestar datos de prueba", "ingest sample data", "populate table", "seed the table", "datos de testing", "quiero datos en la tabla", "ingestar en la tabla".
---

# Hydrolix: Ingest Sample Data

> **HELPER SCRIPT**: All Python logic lives in `.claude/skills/hdx_helpers.py`. Call it as:
> `python3 .claude/skills/hdx_helpers.py <command> [args...]`

Generate and ingest N rows of test data into a Hydrolix table, based on the transform's `sample_data` with timestamps distributed across a given date range.

## Step 1 — Collect inputs

Use `AskUserQuestion` to collect (all in one message):
- **Cluster URL** — e.g. `https://marketplace-testing-cluster.hdx-aws-dev.net`
- **Project name** — e.g. `siem_project_1`
- **Table name** *(optional)* — leave blank to auto-discover all tables with sample data
- **Transform name** *(optional, required if table name is provided)*
- **Number of rows** — default `10`
- **Start date** — default 30 days ago (`YYYY-MM-DD`)
- **End date** — default today (`YYYY-MM-DD`)

If **table name is blank** → skip Steps 4 and 5, go directly to **Step 4-auto**.

## Step 2 — Extract credentials

```bash
source ~/.zshrc 2>/dev/null
read HDX_USER HDX_PWD < <(python3 .claude/skills/hdx_helpers.py extract-credentials <cluster_url>)
```

Stop and report if output is an ERROR line.

## Step 3 — Get JWT

```bash
source ~/.zshrc 2>/dev/null
JWT=$(python3 .claude/skills/hdx_helpers.py get-jwt <cluster_url> <hdx_user> <hdx_pwd>)
echo "✓ JWT obtained: ${JWT:0:20}..."
```

## Step 4 — Discover table auth requirements

```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py discover-table-auth \
  <cluster_url> "$JWT" <project_name> <table_name>
```

Capture `TOKEN_AUTH` and `INGEST_TOKEN` from output.

→ `✓ Table auth discovered (token_auth=<TOKEN_AUTH>)`

## Step 4-auto — Auto-discover tables and ingest (when no table name was provided)

Skip this step if the user specified a table name. Replaces Steps 4, 5, and 6 entirely.

```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py auto-ingest \
  <cluster_url> "$JWT" <project_name> <n_rows> <start_date> <end_date>
```

→ Go to Step 7 (Done).

## Step 5 — Find sample data and primary timestamp field

```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py find-sample-data <transform_name>
```

Capture `TIMESTAMP_FIELD` and `SAMPLE_SOURCE` from output.

→ `✓ Sample data loaded from <SAMPLE_SOURCE> | timestamp field: <TIMESTAMP_FIELD>`

## Step 6 — Generate rows and ingest

```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py ingest \
  <cluster_url> "$JWT" <project_name> <table_name> <transform_name> \
  <n_rows> <start_date> <end_date> <INGEST_TOKEN>
```

## Step 7 — Done

Show:
> "`<n_rows>` rows ingested into `<project_name>.<table_name>` on `<cluster_url>`"
> "Transform: `<transform_name>` | Timestamps: `<start_date>` → `<end_date>`"