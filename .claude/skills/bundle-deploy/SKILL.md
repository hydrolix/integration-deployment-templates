---
name: bundle-deploy
description: Create a new project on a Hydrolix cluster and apply any bundle from latest_versions.yaml to it, including Grafana org and dashboards. Trigger when user wants to create a project + apply a bundle on any cluster.
allowed-tools: Bash, AskUserQuestion
---

# Bundle Deploy

Deploy a new project on a Hydrolix cluster with `trafficpeak_default_shared` + a user-chosen bundle, including the matching Grafana org and dashboards.

> **HELPER SCRIPT**: All Python logic lives in `.claude/skills/hdx_helpers.py`. Call it as:
> `python3 .claude/skills/hdx_helpers.py <command> [args...]`

## Step 1 — Collect inputs

Use `AskUserQuestion` to ask ALL of the following in a single message:

- **Project name**: alphanumeric and underscores only, must start with a letter
- **Bundle name**: must exist in `data/bundles/latest_versions.yaml` (e.g. `trafficpeak_siem`, `trafficpeak_security`)
- **Credentials mode**: `env` (read from environment variables) or `manual` (provide credentials now)
  - If `env`: ask for **cluster URL** and **Grafana URL** (to know which key to look up in env vars)
  - If `manual`: ask for **cluster URL**, **HDX user**, **HDX password**, **Grafana URL**, **Grafana user**, **Grafana password**
- **Rows to ingest** (default: `10`): number of sample rows to ingest after deploy

Capture: `<project_name>`, `<bundle_name>`, `<credentials_mode>`, `<cluster_url>`, `<grafana_url>`, `<hdx_user>`, `<hdx_pwd>`, `<grafana_user>`, `<grafana_pwd>`, `<n_rows>`

## Step 2 — Validate bundle + resolve credentials

**2a. Validate bundle:**
```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py validate-bundle <bundle_name>
```
Capture bundle path from `OK:<path>` as `<bundle_id>`. Stop and report if ERROR.

**2b. Resolve credentials** (only when `<credentials_mode>` is `env`):
```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py extract-credentials <cluster_url> <grafana_url>
```
Capture: line 1 → `<hdx_user>`, line 2 → `<hdx_pwd>`, line 3 → `<grafana_user>`, line 4 → `<grafana_pwd>`.
If `manual`: use values from Step 1 directly.

**2c. Initialize deploy report:**
Determine `<user_bundle>` = the OS user running the skill (use `whoami` or derive from context).
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report init <project_name> <bundle_name> <bundle_id> <user_bundle>
```

→ `✓ Bundle '<bundle_name>' found | credentials resolved | report initialized`

## Step 3 — Generate ingest user

```bash
python3 -c "
import secrets, string
suffix = ''.join(secrets.choice(string.ascii_lowercase + string.digits) for _ in range(8))
print(f'hydrolix.ingest.{suffix}@hydrolix.io')
"
```
Capture as `<ingest_email>`. Customer email: `<project_name>@hydrolix.io`.

## Step 4 — Phase 1: local operations

Run each in sequence. Print `✓` after each success. Stop on any failure.

**4a. Init project**
```bash
uv run cli init project \
  -cu <cluster_url> \
  -p <project_name> \
  --ingest-user <ingest_email> dummy_placeholder
```
→ `✓ Project <project_name> initialized locally`

**4b. Remove ingest user password from .hdc.yaml**
```bash
python3 .claude/skills/hdx_helpers.py clean-hdc-password <cluster_url>
```

**4c. Apply trafficpeak_default_shared (Hydrolix)** — skip if `<bundle_name>` is `trafficpeak_default_shared`
```bash
uv run cli apply bundle -cu <cluster_url> -p <project_name> -n trafficpeak_default_shared
```
→ `✓ Bundle trafficpeak_default_shared applied (Hydrolix)`

**4d. Apply <bundle_name> (Hydrolix)** — skip if `<bundle_name>` is `trafficpeak_default_shared`
```bash
uv run cli apply bundle -cu <cluster_url> -p <project_name> -n <bundle_name>
```
→ `✓ Bundle <bundle_name> applied (Hydrolix)`

**4e. Init Grafana org**
```bash
uv run grafana init organization \
  -cu <grafana_url> \
  -o <project_name> \
  -ce <project_name>@hydrolix.io \
  -hc <cluster_url> \
  -p <project_name> \
  -u "<hdx_user>" \
  -pwd "<hdx_pwd>"
```
→ `✓ Grafana org initialized locally`

**4f. Remove secureJsonData from .gfo.yaml**
```bash
python3 .claude/skills/hdx_helpers.py clean-gfo-credentials <grafana_url> <project_name>
```

**4g. Apply trafficpeak_default_shared (Grafana)** — skip if `<bundle_name>` is `trafficpeak_default_shared`
```bash
uv run grafana apply bundle -cu <grafana_url> -o <project_name> -p <project_name> -n trafficpeak_default_shared
```
→ `✓ Bundle trafficpeak_default_shared applied (Grafana)`

**4h. Apply <bundle_name> (Grafana)** — skip if `<bundle_name>` is `trafficpeak_default_shared`
```bash
uv run grafana apply bundle -cu <grafana_url> -o <project_name> -p <project_name> -n <bundle_name>
```
→ `✓ Bundle <bundle_name> applied (Grafana)`

**4i. Update report — shared_config PASS**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> shared_config PASS
```

## Step 4.5 — Validate transforms

Invoke **`hdx-validate-transforms`** skill:
- Cluster URL: `<cluster_url>`
- Project name: `<project_name>`
- Credentials: already resolved (`<hdx_user>`, `<hdx_pwd>`)

Capture `FIELDS_CHECKED:<N>` from the local validation output.

**If validation PASSED:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> transform_validate PASS fields_checked=<N>
```

**If validation FAILED:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> transform_validate FAIL fields_checked=<N>
```
Show errors and ask user to abort or continue. Do NOT edit bundle source files — report the error location instead.

## Step 5 — Confirmation before push

Use `AskUserQuestion`:
> "All local resources prepared for `<project_name>` (bundle: `<bundle_name>`). Ready to push to both clusters? (yes/no)"

If no → stop.

## Step 6 — Phase 2: push

**6a. Push Hydrolix**
```bash
uv run cli push \
  -c <cluster_host> \
  -f <cluster_host> <project_name> \
  -u "<hdx_user>" -p "<hdx_pwd>"
```
Where `<cluster_host>` = hostname extracted from `<cluster_url>`.

**If push succeeds:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> deploy PASS
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> transform_apply PASS
```

**If push fails:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> deploy FAIL
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> transform_apply FAIL
```

→ `✓ Project <project_name> pushed to <cluster_host>`

**6b. Push Grafana**
```bash
uv run grafana push \
  -c <grafana_host> \
  -u "<grafana_user>" -p "<grafana_pwd>"
```
Note: SecretsManager errors for OTHER orgs are expected — ignore them.

→ `✓ Grafana org <project_name> pushed`

**6c. Patch Grafana datasource**
```bash
source ~/.zshrc 2>/dev/null
python3 .claude/skills/hdx_helpers.py patch-grafana-ds \
  <grafana_url> "<grafana_user>" "<grafana_pwd>" \
  <project_name> <cluster_url> "<hdx_user>" "<hdx_pwd>"
```

## Step 7 — Ingest sample data

Invoke **`hdx-ingest-sample-data`** skill in auto-discovery mode:
- Cluster URL: `<cluster_url>`
- Project name: `<project_name>`
- Table name: *(blank — auto-discover)*
- Number of rows: `<n_rows>`
- Start date: 30 days ago / End date: today

Capture total rows ingested and rows queried from the output.

**If ingestion succeeds:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> roundtrip PASS rows_ingested=<rows_ingested> rows_queried=<rows_queried>
```

**If ingestion fails:**
```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report update <project_name> roundtrip FAIL rows_ingested=<rows_ingested> rows_queried=0
```

→ `✓ Sample data ingested`

## Step 8 — Finalize report and show summary

```bash
uv run python3 .claude/skills/hdx_helpers.py deploy-report finalize <project_name>
```

Show the full JSON report output, plus:
> "✓ Project `<project_name>` fully deployed!"
>
> - Hydrolix: `<cluster_url>` — bundle: `<bundle_name>`
> - Grafana: `<grafana_url>` — org `<project_name>`
> - Ingest user: `<ingest_email>`
> - Report: `data/reports/<project_name>_deploy_report.json`