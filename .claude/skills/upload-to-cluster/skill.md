---
name: upload-to-cluster
description: Batch upload Hydrolix functions and dictionaries to a cluster via the config API. Use when the user wants to upload functions, dictionaries, or both to a Hydrolix cluster.
user-invocable: true
---

# Upload Functions & Dictionaries to Cluster

You are helping batch-upload Hydrolix functions and/or dictionaries to a cluster. Follow this process systematically.

## Phase 1: Gather Inputs

Ask the user for the following. If any are already provided (e.g., as arguments), skip those prompts.

| Input | Required | Notes |
|-------|----------|-------|
| Cluster URL | Yes | e.g., `https://my-cluster.hdx-aws-dev.net` — no trailing slash |
| Username | Yes | Hydrolix cluster credentials |
| Password | Yes | **Never write to disk** |
| Source path | Yes | Directory containing `functions/` and/or `dictionaries/` subdirs, OR a single file |
| Project name(s) | Yes | One or more project names (will be resolved to UUIDs) |

## Phase 2: Authenticate

Log in to obtain a bearer token. Store the token in a shell variable only — never write it to disk.

```bash
TOKEN=$(curl -s -X POST "${CLUSTER}/config/v1/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\": \"${USERNAME}\", \"password\": \"${PASSWORD}\"}" \
  | jq -r '.auth_token.access_token')
```

**Verify the token is not empty.** If login fails, stop immediately and show the error. Do not proceed.

## Phase 3: Discover Org & Resolve Projects

### Get the org

```bash
curl -s "${CLUSTER}/config/v1/orgs/" \
  -H "Authorization: Bearer ${TOKEN}"
```

- If one org: auto-select it and extract `uuid` and `name`.
- If multiple orgs: ask the user which one to use.

### Resolve project names to UUIDs

```bash
curl -s "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/" \
  -H "Authorization: Bearer ${TOKEN}"
```

- Match each user-provided project name to a project in the response.
- Extract the `uuid` for each matched project.
- Warn if any project name doesn't match and ask user whether to continue.

## Phase 4: Scan Source Directory

Classify every file found under the source path:

### Function files

Look in `functions/` and `functions/.extracted/` directories.

A **function** is a `.json` file with `name` + `sql` keys:

```json
{
  "description": "Breadcrumbs extraction",
  "name": "breadcrumbs",
  "sql": "(breadcrumbs, mainregex, valuextract) -> nullIf(...)"
}
```

### Dictionary files

Look in `dictionaries/` and `dictionaries/.extracted/` directories.

Dictionaries consist of a **definition** (JSON) and a **data file** (CSV/YAML/YML/TSV).

A **dictionary definition** is a `.json` file with `name` + `settings` keys:

```json
{
  "name": "geoip_asn_blocks_ipv4",
  "settings": {
    "filename": "geoip_asn_blocks_ipv4",
    "layout": "ip_trie",
    "lifetime_seconds": 5,
    "output_columns": [
      {"name": "network", "datatype": {"type": "string", "denullify": true}},
      {"name": "autonomous_system_number", "datatype": {"type": "uint32", "denullify": true}}
    ],
    "primary_key": ["network"],
    "format": "CSVWithNames"
  }
}
```

A **dictionary data file** is the matching CSV, YAML, YML, or TSV file. Match data files to definitions by:
1. **Same stem**: `geoip_asn_blocks_ipv4.json` pairs with `geoip_asn_blocks_ipv4.csv`
2. **`settings.filename` field**: If the definition has `settings.filename`, use that to match the data file stem.
3. **Manual mapping**: If unmatched, ask the user to specify which data file goes with which definition.

**Alternative format — schema-only definitions**: Some dictionaries use subdirectories where `schema_definition.json` is an array of column definitions alongside the data file:

```
dictionaries/
  geoip_asn_blocks_ipv4/
    schema_definition.json    # Array of column defs
    geoip_asn_blocks_ipv4.csv # Data file
```

Handle both layouts.

### Zip archives

If `dictionaries/dictionaries.zip` or `functions/functions.zip` exists, extract to `.extracted/` first:

```bash
unzip -o dictionaries/dictionaries.zip -d dictionaries/.extracted/
```

### Skip non-relevant files

Skip README, LICENSE, .gitignore, etc. with a warning.

### Present inventory

Before uploading, display the full inventory to the user for confirmation:

```
Functions to upload:
  1. breadcrumbs
  2. city_name
  3. geoname_id

Dictionaries to upload:
  1. geoip_asn_blocks_ipv4 (data: geoip_asn_blocks_ipv4.csv)
  2. ua_cat_dict (data: ua_cat_dict.yml)

Target: cluster.example.com
Org: my-org
Projects: commons, akamai

Proceed? (y/n)
```

## Phase 5: Check for Duplicates

For each target project, fetch existing functions and dictionaries:

```bash
# List existing functions
curl -s "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/" \
  -H "Authorization: Bearer ${TOKEN}"

# List existing dictionaries
curl -s "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/" \
  -H "Authorization: Bearer ${TOKEN}"

# List existing dictionary files
curl -s "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/files/" \
  -H "Authorization: Bearer ${TOKEN}"
```

The response may be a JSON array directly, or an object with a `results`, `functions`, `dictionaries`, or `data` key containing the array. Handle all these shapes.

Mark items that already exist as "will skip" and show the user which items will be skipped.

## Phase 6: Upload Functions

For each function, for each target project:

```bash
curl -s -X POST "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d @function.json
```

The JSON body must include `name`, `sql`, and optionally `description`.

**Template variable replacement**: Before uploading, replace `__PROJECT_NAME__` in the `sql` field with the actual project name.

Track results: uploaded, skipped (duplicate), failed.

## Phase 7: Upload Dictionary Data Files

**This must happen BEFORE Phase 8** — definitions reference data files.

For each dictionary data file, for each target project:

```bash
curl -s -X POST "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/files/" \
  -H "Authorization: Bearer ${TOKEN}" \
  -F "name=${DATA_FILE_STEM}" \
  -F "file=@${DATA_FILE_PATH};type=${MIME_TYPE}"
```

Where:
- `DATA_FILE_STEM` is the filename without extension (e.g., `geoip_asn_blocks_ipv4`)
- `MIME_TYPE` is `text/csv` for CSV/TSV files or `application/x-yaml` for YAML/YML files

Track results: uploaded, skipped (duplicate), failed.

## Phase 8: Upload Dictionary Definitions

For each dictionary definition, for each target project:

```bash
curl -s -X POST "${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d @definition.json
```

The JSON body must include `name` and `settings` (with `filename`, `layout`, `output_columns`).

Track results: uploaded, skipped (duplicate), failed.

## Phase 9: Summary Report

Display:

```
============================================================
Upload Summary — {cluster}
Org: {org_name}
Projects: {project_names}
============================================================

--- Functions ---
  Uploaded: N   Skipped: N (exist)   Failed: N

--- Dictionary Data Files ---
  Uploaded: N   Skipped: N (exist)   Failed: N

--- Dictionary Definitions ---
  Uploaded: N   Skipped: N (exist)   Failed: N

Totals: N uploaded, N skipped, N failed
============================================================
```

If any items failed, list them with the error details.

## Error Handling

| Error | Behavior |
|-------|----------|
| Login failure | Stop immediately, show error |
| 401 mid-batch | Re-authenticate (repeat Phase 2), retry the failed request once |
| 400/409 (duplicate) | Skip item, count as "skipped" in summary |
| 400 (other) | Log response body, mark failed, continue |
| 500 / network error | Retry once, then mark failed, continue |
| File read error | Skip file, warn, continue |

**Guiding principle:** Never let a single item failure abort the entire batch.

## Security Rules

- **Never** write credentials or tokens to disk
- Use `curl -s` to suppress progress output that might leak info
- Store bearer token only in shell variables
- Do not echo passwords in output
