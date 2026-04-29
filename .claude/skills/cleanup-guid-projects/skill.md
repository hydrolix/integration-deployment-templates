---
name: cleanup-guid-projects
description: Bulk-delete ephemeral test projects on a Hydrolix cluster (default pattern `bundle_verification_*`, produced by the Rust validator's `--guid` flag). Lists matching projects, confirms with the user, then deletes sequentially. Use when the user wants to clean up leftover GUID test projects, or projects matching a name prefix, on a cluster.
user-invocable: true
---

# Cleanup GUID Test Projects

Bulk-deletes Hydrolix projects whose names match a prefix on a Hydrolix cluster. Built for cleaning up the ephemeral `bundle_verification_<suffix>` projects produced by `cargo run -- --local --guid`, but the prefix is configurable.

## When to use

- User says they've accumulated a pile of `bundle_verification_*` projects from local `--guid` runs.
- User wants to bulk-delete any set of projects on a cluster by name prefix.
- Any variant: "clean up test projects", "delete the guid projects", "bulk delete projects on demo".

If the user only wants to delete ONE project, defer to `cargo run --bin cleanup -- --project <name>` instead — no need for this skill.

## Phase 1: Gather inputs

Collect these values — use defaults when the user doesn't override:

| Input | Default | Notes |
|-------|---------|-------|
| Prefix | `bundle_verification_` | Only projects whose `.name` starts with this prefix are candidates. |
| Cluster | `$BUNDLE_TESTING_CLUSTER` env var | Hostname without scheme (e.g., `demo.aws.hydrolix.live`). |
| Username | `$BUNDLE_TESTING_USERNAME` | |
| Password | `$BUNDLE_TESTING_PASSWORD` | **Never write to disk.** |
| Org UUID | `2b8cbbf8-dcb8-4c28-bd94-cb46147296d1` (demo.aws.hydrolix.live) | Only correct for demo cluster. For other clusters, discover via `GET /config/v1/orgs/`. |
| `--force` | off | If set, skip the confirmation prompt. Only pass when the user explicitly asked for non-interactive mode. |

If any env var is unset, ask the user.

## How to carry state across Bash calls

Each Bash tool call runs in a fresh shell — `$TOKEN` and other variables set in one call do NOT survive to the next. **Do not work around this by writing the token to a file** (that violates the security rule below).

The right pattern: re-authenticate at the top of any Bash call that needs the token. Auth is ~1s and idempotent. Concretely:

- Combine Phase 2 (auth) + Phase 3 (list) into a single Bash call so `$TOKEN` is in scope for both.
- Phase 5 (delete) is its own Bash call — re-auth from `$BUNDLE_TESTING_USERNAME` / `$BUNDLE_TESTING_PASSWORD` at the top.
- Phase 6 (verify) can either run in the same call as Phase 5 (preferred) or re-auth.

The TSV at `/tmp/cleanup_targets.tsv` is fine to leave on disk between calls — it only contains public project names + UUIDs.

## Phase 2: Authenticate

```bash
TOKEN=$(curl -sf --max-time 30 -X POST "https://${CLUSTER}/config/v1/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${BUNDLE_TESTING_USERNAME}\",\"password\":\"${BUNDLE_TESTING_PASSWORD}\"}" \
  | jq -r '.auth_token.access_token')
```

Verify `$TOKEN` is non-empty and not `null`. If auth fails, stop and show the error.

If the user's cluster is not `demo.aws.hydrolix.live`, discover the org UUID:

```bash
curl -sf --max-time 15 -H "Authorization: Bearer $TOKEN" \
  "https://${CLUSTER}/config/v1/orgs/" | jq -r '.[0].uuid'
```

If multiple orgs, ask which to use.

## Phase 3: List matching projects

```bash
curl -sf --max-time 15 -H "Authorization: Bearer $TOKEN" \
  "https://${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/" \
  | jq -r --arg p "$PREFIX" '(.results // .) | .[] | select(.name | startswith($p)) | "\(.name)\t\(.uuid)"' \
  > /tmp/cleanup_targets.tsv
TOTAL=$(wc -l < /tmp/cleanup_targets.tsv | tr -d ' ')
```

Handle both response shapes: top-level array *or* `{results: [...]}`.

Show the user the count and the first ~20 names. Full list is in `/tmp/cleanup_targets.tsv`.

If `TOTAL == 0`, stop and tell the user there's nothing to delete.

## Phase 4: Confirm (unless `--force`)

Print the cluster name, the prefix, the count, and the exact endpoint that will be used (`DELETE https://${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/<uuid>`). Ask the user to say **proceed** before deleting. This is destructive and affects shared cluster state.

Skip this prompt only if the user explicitly asked for `--force` or a non-interactive run.

## Phase 5: Delete sequentially

**Use this exact loop pattern** — `while IFS=$'\t' read -r ... < file` with `curl --max-time 60`. Do NOT use `cat file | xargs -P N bash -c '...'` or process substitution `< <(...)` — those patterns hang silently inside the Bash tool for 10+ minutes with zero output (see `memory/feedback_bash_loop_patterns.md`).

The snippet re-authenticates at the top so it's self-contained and the token never has to cross Bash invocations:

```bash
TOKEN=$(curl -sf --max-time 30 -X POST "https://${BUNDLE_TESTING_CLUSTER}/config/v1/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${BUNDLE_TESTING_USERNAME}\",\"password\":\"${BUNDLE_TESTING_PASSWORD}\"}" \
  | jq -r '.auth_token.access_token')
[ -z "$TOKEN" ] || [ "$TOKEN" = "null" ] && { echo "AUTH FAILED"; exit 1; }
TOTAL=$(wc -l < /tmp/cleanup_targets.tsv | tr -d ' ')

OK=0; FAIL=0; i=0
while IFS=$'\t' read -r NAME UUID; do
  i=$((i+1))
  code=$(curl -s --max-time 60 -o /dev/null -w '%{http_code}' \
    -X DELETE -H "Authorization: Bearer $TOKEN" \
    "https://${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${UUID}")
  case "$code" in
    204|200) printf "[%2d/%d] ✓ %s\n" "$i" "$TOTAL" "$NAME"; OK=$((OK+1));;
    404)     printf "[%2d/%d] — %s (already gone)\n" "$i" "$TOTAL" "$NAME"; OK=$((OK+1));;
    *)       printf "[%2d/%d] ✗ %s HTTP=%s\n" "$i" "$TOTAL" "$NAME" "$code"; FAIL=$((FAIL+1));;
  esac
done < /tmp/cleanup_targets.tsv
echo "summary: ${OK}/${TOTAL} deleted, ${FAIL} failed"
```

Timing: one delete is typically ~1s, so 50 projects ≈ under a minute. If the loop slows noticeably (many 5s+ deletes), the cluster may be cascade-deleting large tables — let it finish.

## Phase 6: Verify

Re-query the cluster and assert the count is zero:

```bash
curl -sf --max-time 15 -H "Authorization: Bearer $TOKEN" \
  "https://${CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/" \
  | jq -r --arg p "$PREFIX" '(.results // .) | .[] | select(.name | startswith($p)) | .name' | wc -l
```

If non-zero, report which names remain and suggest re-running.

## Error handling

| Error | Behavior |
|-------|----------|
| Auth failure | Stop immediately, show response body. |
| List API failure | Stop — don't guess the project list. |
| `404` on DELETE | Count as success (project already gone). |
| `5xx` or network timeout on DELETE | Log, mark failed, continue with next. |
| 5+ consecutive failures | Stop and report — the cluster may be down. |

## Security

- Never write `BUNDLE_TESTING_PASSWORD` or `$TOKEN` to disk. If you need the token in a later Bash call, re-authenticate (see "How to carry state across Bash calls" above) — auth is ~1s and idempotent, there's no reason to persist the token.
- Use `curl -s` (silent) so progress output doesn't leak headers.
- The target list TSV only contains public project names + UUIDs — safe to keep in `/tmp`.

## Why not just `cargo run --bin cleanup -- --project <name>` in a loop?

That works for a handful, but (a) re-authenticates each iteration and (b) rebuilds aren't the bottleneck but each invocation has startup overhead. For N>5 this skill is faster and shows unified progress. For one project, prefer the Rust binary.
