# How to Contribute Integrations

This guide outlines the process for external teams to contribute integration assets to the Hydrolix console.

## Contribution Workflow

### 1. Create a Jira Ticket

Create a Jira ticket in the **LOTC** project for your integration request. Use the "Bundle Request" work type.

**Jira Project**: [LOTC Board](https://hydrolix.atlassian.net/jira/software/projects/LOTC/boards/635)

Include the following information in your ticket:

#### Required Information

**Basic Details**
- **Integration name**: Technical name (lowercase, underscores allowed, e.g., `cdn_insights`)
- **Display title**: User-facing name (e.g., "CDN Insights")
- **Description**: Clear explanation of what the integration does and its purpose
- **Category**: AWS or TrafficPeak
- **Data category**: Type of data (e.g., security, observability, analytics)

**Table Configuration**
- **Primary table name**: Name for the main table (e.g., `cdn_insights_logs`)
- **Summary table names** (if applicable): Names for any summary/aggregation tables

**UI Assets**
- **Integration logo**: PNG image or URL for the integration icon
- **Method logo** (if applicable): Icon representing the data ingestion method
- **Documentation URL**: Link to integration documentation (if available)

**Technical Details**
- **Ingestion method**: How data will be ingested (e.g., `http_streaming`, `firehose`, `multi_stream`)
- **Dependencies**: Any shared dictionaries or functions required (e.g., GeoIP, user-agent parsing)
- **New or updated dictionaries**: If providing new dictionaries or updates to existing ones, specify the dictionary names and describe the changes
- **New or updated functions**: If providing new functions or updates to existing ones, specify the function names and describe their purpose
- **Sample data**: Representative data samples for each transformation

#### Optional Information
- **Maintainer contact**: Email address for the integration maintainer
- **Beta status**: Whether this integration is in beta
- **Multiple dashboards**: If providing multiple dashboard views (e.g., overview, detailed, raw)

### 2. Create a Branch

Create a branch named with your ticket number and a brief description:

```bash
git checkout main
git pull origin main
git checkout -b TICKET-123-integration-name
```

**Example:** `JIRA-456-cloudflare-logs`

### 3. Prepare and Load Assets

Organize your assets following the bundle structure. Assets should be placed in a newly created folder under either `aws/` or `trafficpeak/` depending on your integration category.

#### Repository Structure

Assets are placed directly under your project name — no version subdirectory.

```
integration-deployment-templates/
├── aws/
│   └── your-project-name/
│       ├── bundle-config.json    ← required
│       ├── dashboards/
│       ├── dictionaries/
│       ├── functions/
│       ├── transformations/
│       └── summaries/
└── trafficpeak/
    └── your-project-name/
        ├── bundle-config.json    ← required
        ├── dashboards/
        ├── dictionaries/
        ├── functions/
        ├── transformations/
        └── summaries/
```

For example, if you're contributing a CloudFlare integration under AWS, your assets go in `aws/cloudflare/`.

#### `bundle-config.json` (Required)

Every bundle directory must include a `bundle-config.json` file. This file is used by CI to automatically format, configure, and validate your bundle when you open a PR.

**Required fields:**

```json
{
  "table_name": "your_table_name",
  "data_category": "security",
  "version": "1.0.0"
}
```

| Field | Description | Values |
|-------|-------------|--------|
| `table_name` | Primary table name. Letters, digits, and underscores only (no dashes). | e.g., `bot_detection`, `cdn_logs`, `siem_logs` |
| `data_category` | Type of data the integration handles. | `video`, `cdn`, `security`, `api`, or `dns` |
| `version` | Bundle version. Must match the version folder name. | e.g., `1.0.0`, `1.1.0`, `2.0.0` |

**Optional fields** (override auto-inferred values):

| Field | Description |
|-------|-------------|
| `source_name` | Source provider name (default: inferred from directory path) |
| `bundle_name` | Bundle identifier (default: inferred from directory name) |
| `channel_type` | `AWS`, `Azure`, `GCP`, `3rdParty`, or `Internal` (default: auto-detected) |
| `method` | Ingestion method: `http_streaming`, `firehose`, `kinesis`, `multi_stream`, etc. (default: auto-detected) |
| `description` | Integration description (default: auto-generated) |
| `beta` | Whether this is a beta integration (default: `true`) |

**Grafana folder placement:**

Dashboards are automatically placed in the correct Grafana folder based on `data_category`. No additional fields are needed.

| `data_category` | Grafana folder |
|---|---|
| `security` | TrafficPeak Certified Reference Dashboards → Security |
| `cdn` | TrafficPeak Certified Reference Dashboards → CDN |
| `video` | TrafficPeak Certified Reference Dashboards → Media |
| `api` | TrafficPeak Certified Reference Dashboards → API Context |
| `dns` | TrafficPeak Certified Reference Dashboards → DNS |

> **Note:** For new bundles with raw assets, do not create a `bundle.json` file manually — it is auto-generated by CI from your assets and `bundle-config.json`. If you are submitting pre-configured assets that have already been formatted, you must include a valid `bundle.json` alongside them (see [Common pitfalls](#common-pitfalls) below).

#### Asset Types and Formats

**Dashboards**
- Grafana JSON dashboard files
- Include primary dashboard and any additional views (global, detailed, raw, etc.)
- Place in `dashboards/` folder

**Dictionaries**
- CSV files with key-value mappings
- Used for lookups and data enrichment
- Place in `dictionaries/` folder

**Functions**
- SQL function definitions
- Custom functions used in transformations or queries
- Place in `functions/` folder

**Transformations**
- Transformation configuration JSON files
- **Must include sample data** (`sample_data.json`) for each transformation
- Organize by ingestion method or data source (e.g., `transformations/cloudflare/`, `transformations/firehose/`)
- Place in `transformations/` folder

**Summaries**
- SQL files for summary/aggregation table definitions
- Used for pre-computed metrics and roll-ups
- Place in `summaries/` folder

**Logos**
- PNG format recommended
- Minimum 200x200 pixels for clarity
- Transparent background preferred
- Can be provided as files or publicly accessible URLs

### 4. Commit and Push Assets

```bash
git add .
git commit -m "Add [integration name] assets for TICKET-123"
git push origin TICKET-123-integration-name
```

### 5. Create a Pull Request

Create a pull request to the `main` branch for asset review:

1. Go to the repository on GitHub
2. Create a new Pull Request from your branch to `main`
3. Use the title format: `[TICKET-123] Integration Name`
4. In the PR description:
   - Link to your Jira ticket
   - Summarize the integration and assets provided
   - Note any dependencies or special considerations
5. Request review from the Integration Engineer

**What happens next:** When your PR is opened, CI automatically determines which pipeline track to run based on what you changed:

- **Track 1 (Full Pipeline)** — runs when raw (unformatted) assets are detected. Backs up your originals to `.originals/`, then runs the full format + validate flow: configures assets, generates `bundle.json`, creates portable bundle artifacts, and commits everything back to your PR branch.
- **Track 2 (Validation Only)** — runs when only configured (already-formatted) assets are changed. Runs structural validation checks without reformatting. Use this path for small fixes like updating a bundle description or fixing a dashboard panel title.

The pipeline auto-detects which track to use — you don't need to set any flags or labels. If detection is ambiguous, it defaults to the safe path (validation only) and adds a PR annotation explaining why.

If either step fails, check the workflow logs for details and fix the issues in your branch.

**Note**: This PR will remain open throughout the process. The Integration Engineer will review the CI results and run additional manual validation before merging.

### 6. Assign to Integration Engineer

Update your Jira ticket and assign it to an integration engineer on the SaaS Engineering team. The team will:
- Review your submitted assets via the PR
- Provide feedback through PR comments and Jira
- Once assets are approved, add bundling work to your branch
- Create the `bundle.json` configuration file
- Adapt assets for validation and deployment
- Run validation checks

### 7. Wait for Feedback

The Integration Engineer will provide feedback through both the Pull Request and Jira ticket. You may receive:

**Technical Validation Feedback**
- Asset format issues
- Schema validation errors
- Configuration problems

**Functional Testing Feedback**
- Runtime errors
- Data processing issues
- Integration compatibility problems

### 8. Update Assets Based on Feedback

If issues are identified with the provided assets:

1. Make necessary corrections to your assets
2. Commit and push updates to your branch
```bash
git add .
git commit -m "Update assets based on feedback for TICKET-123"
git push origin TICKET-123-integration-name
```
3. Comment on the PR and/or Jira ticket to notify the Integration Engineer
4. The PR will automatically update with your new commits

**How CI handles your update:** The pipeline detects what changed and routes automatically:
- **Editing raw assets** (e.g., re-uploading a dashboard or transform): triggers Track 1 — restores from `.originals/`, re-formats everything from scratch, and commits the results. Your raw originals are preserved safely.
- **Editing configured assets** (e.g., fixing a typo in `bundle.json` descriptions, adjusting a dashboard panel title): triggers Track 2 — validates without reformatting, so your changes are preserved as-is.

You don't need to worry about which track runs — the pipeline figures it out. If you're unsure whether your change will trigger a reformat, push it and check the CI logs.

### 9. Bundling and Final Validation

Once your assets are approved:
- The Integration Engineer will continue work on **your branch**
- They will add bundling configuration (`bundle.json`) and any necessary adaptations
- Additional commits will appear in your PR from the Integration Engineer
- The Integration Engineer will run validation and testing
- The PR will be merged to `main` only when the complete integration is validated and ready for deployment

You will receive confirmation via the PR and Jira ticket when the integration is complete.

## Two-Track Pipeline System

The CI pipeline uses a two-track system to handle both new bundle submissions and iterative edits safely.

### Track 1: Full Pipeline (format + validate)

Runs when the pipeline detects **raw, unformatted assets** — either from a first-time submission or from edits to the `.originals/` directory. This track:

1. Backs up raw assets to `.originals/` (first run) or restores from `.originals/` (re-run)
2. Runs Stage 1 (bundle generation) and Stage 2 (configure and format)
3. Generates `bundle.json` and portable bundle artifacts
4. Runs Stage 3 (structural validation)
5. Commits all changes back to the PR branch

### Track 2: Validation Only

Runs when the pipeline detects **already-configured assets** — typically small edits to formatted files. This track:

1. Runs Stage 3 (structural validation) only
2. Does not reformat or regenerate anything
3. Preserves your changes exactly as committed

### How detection works

The pipeline inspects your changed files and checks for markers that indicate raw vs. configured state:
- **Raw markers:** `__inputs` in transforms, bare dashboard JSON (no `{"dashboard": ...}` wrapper), hardcoded SQL table references
- **Configured markers:** template variables (`__DATASOURCE__`, `__TABLE_NAME__`), wrapped dashboards, `bundle.json` present

If signals are mixed or ambiguous, the pipeline defaults to Track 2 (validation only) to avoid unintended reformatting.

### The `.originals/` directory

When Track 1 runs for the first time, it automatically saves a copy of your raw assets to `.originals/`. This enables clean re-runs at any time — the pipeline can always restore the original state and reformat from scratch.

See [`.originals/README.md`](.originals/README.md) for details.

### Emergency overrides

Two override mechanisms exist for exceptional situations:

- **`skip-bundle-ci` label:** Blocks all CI pipeline processing (both tracks). Use only as a last resort if CI is actively broken and you need to merge.

These are escape hatches, not part of normal workflow. The auto-detection system handles routing in all standard cases.

### For internal team members

- To trigger a full re-pipeline, edit files in `.originals/` and push.
- Legacy bundles without `.originals/` always run Track 2. To opt a legacy bundle into Track 1, add a `bundle-config.json` and upload raw assets.
- When reviewing PRs, the CAC test push happens automatically on approval. Re-approving after changes updates the existing CAC test PR (no duplicate PRs).

### Common pitfalls

**Missing `bundle.json` with configured assets.** If you submit assets that are already configured (wrapped dashboards with `{"dashboard": ...}`, template variables like `__DATASOURCE__` in SQL) but without a `bundle.json`, the pipeline will detect configured state and route to Track 2 (validation only). However, the validator discovers bundles through `bundle.json` — without it, your bundle is invisible and CI will fail with an error. To fix this, either:
1. Submit raw/unformatted assets with a `bundle-config.json` so Track 1 generates `bundle.json` automatically, or
2. Include a valid `bundle.json` alongside your configured assets.

**`bundle-config.json` structured as `bundle.json`.** The `bundle-config.json` should contain only simple configuration fields (`table_name`, `data_category`, `version`, `folder`, `subfolder`). Do not put full bundle metadata (`name`, `tables`, `source`, `method`, `dependencies`, `ui`) in `bundle-config.json` — that structure belongs in `bundle.json`, which is auto-generated by the pipeline.

## Best Practices

- **Follow naming conventions**: Use clear, descriptive names for all asset files
- **Test locally**: Validate SQL syntax and JSON formatting before submission
- **Document dependencies**: Note any external dependencies or data requirements
- **Provide examples**: Include sample data or usage examples where applicable
- **Respond promptly**: Address feedback quickly to expedite the integration process
- **Use PR comments**: Ask questions and clarify feedback directly on the PR for better context
- **Monitor your branch**: The Integration Engineer will add commits to your branch—this is expected and part of the workflow

## Need Help?

If you have questions or need assistance:
- Comment on your Jira ticket
- Contact an integration engineer on the SaaS engineering team (Slack: @eng-marketplace)
- Refer to existing integrations in the repository for examples
