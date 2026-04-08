# GitHub Workflows

This directory contains the CI/CD workflows for the integration-deployment-templates repository.

---

## Workflows

### `bundle-ci`

**Trigger:** Pull requests to `main` that touch `aws/**`, `trafficpeak/**`, `portables/**`, or `.originals/**`

The primary CI pipeline for bundle changes. Detects which bundles changed and routes them into one of two tracks:

- **Track 1 — Full Pipeline:** Runs format + configure + validate. Used when a bundle has a `bundle-config.json` and has meaningful source changes. Auto-commits formatted output back to the branch with `[skip ci]`.
- **Track 2 — Validate Only:** Runs validation only, no formatting or commit. Used for bundles without a `bundle-config.json` or where only generated files changed.

Add the `skip-bundle-ci` label to a PR to bypass this workflow entirely.

---

### `bundle-validator`

**Trigger:** Pull requests with the `skip-bundle-format` label or `[skip-bundle-format]` in the PR body; also supports manual dispatch.

Runs the Rust-based `bundle-validator` binary against the repository. Used as a standalone validation step when the full `bundle-ci` pipeline is bypassed.

**Manual override:**

```bash
gh workflow run bundle-validator.yml
```

Or via GitHub UI: **Actions → bundle-validator → Run workflow**

---

### `push-to-cac-tools-test`

**Trigger:** PR approval (`pull_request_review: submitted`); also supports manual dispatch.

When a PR is approved, detects any changed bundles under `portables/` and opens a corresponding PR in the `hydrolix/cac-tools-test` repository for testing. If a PR already exists for that branch, it force-updates it with the latest changes.

**Manual override:**

```bash
gh workflow run push-to-cac-tools-test.yml --field pr_number=<PR_NUMBER>
```

Or via GitHub UI: **Actions → push-to-cac-tools-test → Run workflow → enter PR number**

---

### `push-to-cac-tools`

**Trigger:** PR merged to `main` that touches `portables/**`; also supports manual dispatch.

After a PR is merged, detects changed bundles under `portables/` and opens a PR in the `hydrolix/cac-tools` (PROD) repository to sync the bundle for production deployment.

**Manual override:**

```bash
gh workflow run push-to-cac-tools.yml
```

Or via GitHub UI: **Actions → push-to-cac-tools → Run workflow**

> Note: Manual dispatch runs against the current state of `main`. The resulting CAC PR will be prefixed with `[TEST]` to indicate it was triggered manually rather than by a merge.

---

### `publish-runbook`

**Trigger:** Pushes to `main` that modify `aws/**/bundle.json` or `trafficpeak/**/bundle.json`; also supports manual dispatch.

Automatically generates a structured runbook from `bundle.json` metadata, publishes it to the Confluence Technical Enablement workspace, and sends a Slack notification to `#solutions-bundles-alerts`.

The workflow runs in parallel for each changed bundle using a matrix strategy.

**Manual override — GitHub UI:**

1. Go to **Actions → publish-runbook → Run workflow**
2. Fill in:
   - **Bundle directory path** — e.g. `aws/cdn-insights` or `trafficpeak/default_shared`
   - **Force publish** — set to `true` to republish even if content is unchanged

**Manual override — CLI:**

```bash
gh workflow run publish-runbook.yml \
  --field bundle_path=aws/cdn-insights \
  --field force_publish=true
```

#### Authentication & secrets

This workflow uses **AWS OIDC** — no static AWS credentials are stored in GitHub. Instead, the workflow assumes an IAM role via GitHub's OIDC provider and pulls all secrets from AWS Secrets Manager at runtime.

**GitHub repo variables (Settings → Variables):**

| Variable | Description |
|---|---|
| `AWS_ROLE_ARN` | IAM role ARN the workflow assumes via OIDC |
| `AWS_SECRET_ARN` | ARN of the Secrets Manager secret containing all runtime credentials |

**AWS Secrets Manager secret** (`integration-deployment-templates/env`):

The secret is a JSON object with the following keys:

| Key | Used by |
|---|---|
| `CONFLUENCE_BASE_URL` | `publish_to_confluence.py` |
| `CONFLUENCE_USER` | `publish_to_confluence.py` |
| `CONFLUENCE_API_TOKEN` | `publish_to_confluence.py` |
| `CONFLUENCE_SPACE_KEY` | `publish_to_confluence.py` |
| `CONFLUENCE_PARENT_PAGE_ID` | `publish_to_confluence.py` |
| `SLACK_RUNBOOK_WEBHOOK_URL` | `notify_bundle_team.py` |

To update a secret value, go to **AWS Console → Secrets Manager → `integration-deployment-templates/env` → Retrieve secret value → Edit**.

#### Testing scripts locally

If you need to test `generate_runbook.py`, `publish_to_confluence.py`, or `notify_bundle_team.py` locally, pull the secrets from AWS Secrets Manager directly into your shell. Requires AWS CLI configured with credentials that have `secretsmanager:GetSecretValue` on the secret.

```bash
# Load secrets as environment variables into your current shell
eval $(aws secretsmanager get-secret-value \
  --secret-id "$AWS_SECRET_ARN" \
  --query SecretString \
  --output text | python3 -c "
import json, sys
d = json.load(sys.stdin)
for k, v in d.items():
    print(f'export {k}={v}')
")
```

Or write to a `.env` file (ensure `.env` is in `.gitignore`):

```bash
aws secretsmanager get-secret-value \
  --secret-id "$AWS_SECRET_ARN" \
  --query SecretString \
  --output text | python3 -c "
import json, sys
d = json.load(sys.stdin)
for k, v in d.items():
    print(f'{k}={v}')
" > .env
```

Then run scripts as normal — they will pick up the credentials from the environment.

---

## Required Secrets & Environments

| Environment | Secrets |
|---|---|
| `bundle-validator-env` | `BUNDLE_TESTING_CLUSTER`, `BUNDLE_TESTING_USERNAME`, `BUNDLE_TESTING_PASSWORD` |
| `bundle-runbook-env` | *(none — credentials are pulled from AWS Secrets Manager at runtime)* |

The `push-to-cac-tools` and `push-to-cac-tools-test` workflows use a GitHub App for cross-repo access, configured via `INTEGRATIONS_APP_ID` (variable) and `INTEGRATIONS_APP_PRIVATE_KEY` (secret).
