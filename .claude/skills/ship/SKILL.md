---
name: ship
description: "Commit, push, and open a pull request for the current branch. If the PR targets hydrolix/integration-deployment-templates, also sends a Slack notification to #solutions-bundles-alerts. Use when the user says 'ship it', 'open a PR', 'submit changes', 'push and create pull request', 'send for review', or 'ship this branch'."
---

# Ship

Commit staged/unstaged changes, push the branch, open a PR, and optionally notify Slack.

## Configuration

| Key | Value |
|-----|-------|
| GitHub Org | `hydrolix` |
| Integrations Repo | `hydrolix/integration-deployment-templates` |
| Slack Channel | `#solutions-bundles-alerts` (`C0APNU3FH6U`) |
| Slack URL | `https://hydrolix.slack.com` |

## Instructions

### Step 1: Determine context

Run in parallel:

```bash
git status
git log --oneline -5
git remote -v
git branch --show-current
git rev-parse --abbrev-ref HEAD@{upstream} 2>/dev/null || echo "no-upstream"
```

### Step 2: Commit

If there are uncommitted changes (staged, unstaged, or untracked):
- Run `git status --short` to identify all untracked files
- For each untracked file, read its contents using the Read tool before staging it
- Stage all changes including untracked files: `git add -A`
- Ask the user: "What should the commit message be?" using AskUserQuestion with header "Commit message"
- Commit with the provided message

If everything is already committed, skip to Step 3.

### Step 3: Push

Push the current branch to origin:
- If the branch has no upstream: `git push -u origin HEAD`
- Otherwise: `git push`

### Step 4: Code Review

Use the Agent tool with `subagent_type: "code-reviewer"` to review the branch changes. Pass it:
- The current branch name
- The working directory path
- The commits ahead of main (from `git log main..HEAD --oneline`)

Display the review results to the user. Then ask: "Proceed with opening the PR?" using AskUserQuestion with header "Code review" and options "Proceed" and "Abort".

If the user chooses "Abort", stop here and do not open a PR.

### Step 5: Create PR

Check if a PR already exists for this branch:
- `gh pr view --json url,title 2>/dev/null`

If no PR exists:
- Run `git diff main...HEAD --stat` to get a summary of files changed vs the base branch
- Auto-generate a PR description with a "## Changes" section listing the changed files and a brief summary of what each file does (infer from filename/path if content is already known from Step 2)
- Ask the user: "PR title?" using AskUserQuestion with header "PR title"
- Create the PR: `gh pr create --title "{title}" --body "{auto-generated description}"`
- Capture the PR URL from the output

If a PR already exists, use its URL and title for the next step.

### Step 6: Notify Slack (integrations repo only)

Detect the repo by running `git remote get-url origin` and checking if it contains `integration-deployment-templates`.

If it does:
- Get the latest commit message: `git log -1 --pretty=%s`
- Use `mcp__claude_ai_Slack__slack_send_message` with:
  - `channel_id`: `C0APNU3FH6U`
  - `text`: `{latest commit message} — {PR URL}`

If it does not match the integrations repo, skip this step silently.

### Step 7: Confirm

Print a brief summary:
- Commit hash (if a new commit was made)
- PR URL
- Whether a Slack notification was sent
