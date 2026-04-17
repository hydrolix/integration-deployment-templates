---
name: hdx-validate-transforms
description: Validate Hydrolix transforms for a project before pushing to the cluster. Runs local schema validation (catches invalid datatype names like "float" that only fail at push time) and API validation (catches SQL errors and sample_data ingestion issues). Use before any hdx push, or standalone when the user wants to check transforms, verify datatype correctness, or pre-flight a deployment. Trigger on phrases like "validate transforms", "check transforms", "verify datatypes", "pre-flight validation".
allowed-tools: Bash, AskUserQuestion
---

# Hydrolix: Validate Transforms

Validate all transforms in a local project before pushing to the Hydrolix cluster.

> **HELPER SCRIPT**: All Python logic lives in `.claude/skills/hdx_helpers.py`. Call it as:
> `python3 .claude/skills/hdx_helpers.py <command> [args...]`

## Step 1 — Collect inputs

If not already known, use `AskUserQuestion` to collect:
- **Cluster URL** — e.g. `https://marketplace-testing-cluster.hdx-aws-dev.net`
- **Project name** — e.g. `siem_project_1`
- **Credentials mode**: `env` (read from environment variables) or `manual`
  - If `env`: cluster URL is sufficient (credentials resolved from env)
  - If `manual`: ask for **HDX user** and **HDX password**

Derive `<cluster_host>` = hostname extracted from `<cluster_url>` (strip `https://`).

## Step 2 — Resolve credentials

**If `env` mode:**
```bash
source ~/.zshrc 2>/dev/null
read HDX_USER HDX_PWD < <(python3 .claude/skills/hdx_helpers.py extract-credentials <cluster_url>)
```
Stop and report if output is an ERROR line.

**If `manual` mode:** use values from Step 1 directly as `<hdx_user>` and `<hdx_pwd>`.

## Step 3 — Local schema validation

```bash
cd /Users/lorenzattileo/PycharmProjects/cac-tools-test
python3 .claude/skills/hdx_helpers.py validate-transforms-local <cluster_host> <project_name>
```

- Scans all transform JSON files referenced by the project hdp.yaml (following `__extend__` chains)
- Checks every `output_columns[].datatype.type` against the valid Hydrolix type set (loaded dynamically from `docs/transform_validations.json`)
- Output includes `TYPES_SOURCE:file` (loaded from JSON) or `TYPES_SOURCE:fallback` (using hardcoded set) and `FIELDS_CHECKED:<N>` for traceability
- Output lines starting with `FAILED:` indicate invalid types

**If any FAILED lines are found:**
- Show each FAILED line clearly
- Use `AskUserQuestion`:
  > "Local schema validation found the above errors. Do you want to abort and fix them, or continue to API validation anyway? (abort/continue)"
- If `abort` → stop. If `continue` → proceed to Step 4.

**If output is `OK: all transforms valid`:**

→ `Local schema validation passed`

## Step 4 — API validation

```bash
source ~/.zshrc 2>/dev/null
uv run cli validate transforms \
  -c <cluster_host> \
  -u "<hdx_user>" -p "<hdx_pwd>" \
  --details error
```

Parse the output:
- Lines containing `FAILED` indicate transform errors (SQL errors, indexer errors, sample_data issues)
- Lines containing `OK` or `PASSED` indicate success

**If any FAILED lines are found:**
- Show each FAILED line clearly
- Note: these errors would prevent a successful push

**If no FAILED lines:**

→ `API validation passed`

## Step 5 — Summary

Show a final summary:

- **PASS**: both validations clean → "All transforms valid for `<project_name>` on `<cluster_host>`. Safe to push."
- **FAIL (local only)**: "Local schema errors found — fix invalid datatype names before pushing."
- **FAIL (API only)**: "API validation errors found — check SQL/sample_data issues before pushing."
- **FAIL (both)**: "Both local and API validation failed — fix all errors before pushing."