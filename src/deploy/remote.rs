// --remote review-deploy orchestrator.
//
// Pushes a bundle's tables, transforms and summaries to the demo Hydrolix
// cluster (creating or reusing a project named after the bundle), then
// upserts dashboards into the persistent review Grafana via direct HTTP.
//
// Validation runs upstream in main.rs and is fail-fast — by the time we get
// here, the bundle is known good.
//
// Sample-data ingestion is intentionally skipped: real data flows in via
// separate ingestion pipelines, and re-ingesting test fixtures would
// pollute reviewer-visible data.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use tokio::time::sleep;

use bundle_validator::remote::dashboard_rewrite::rewrite_datasource_uid;
use bundle_validator::remote::sanitize::sanitize_project_name;

use crate::grafana::remote::{ensure_subfolder, upsert_dashboard, RemoteConfig};
use crate::hdx;
use crate::models::bundle::Bundle;
use crate::DELAY_MODE;

const TABLE_READY_DELAY_SECS: u64 = 30;
const TABLE_PROPAGATION_DELAY_SECS: u64 = 15;
const TABLE_PROPAGATION_DELAY_SLOW_SECS: u64 = 45;
const TRANSFORM_READY_DELAY_SECS: u64 = 15;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let project_name = sanitize_project_name(&bundle.name)
        .map_err(|e| format!("invalid bundle name '{}': {e}", bundle.name))?;

    let cfg = RemoteConfig::from_env()?;

    println!(
        "--- --remote: pushing bundle '{}' as project '{}'",
        bundle.name, project_name
    );

    let bearer_token = hdx::auth::get_token()
        .await
        .map_err(|e| format!("auth failed: {e}"))?;

    ensure_project(&bearer_token, &project_name).await?;
    push_hydrolix(base, bundle, &bearer_token, &project_name).await?;
    let folder_uid = push_grafana(base, bundle, &project_name, &cfg).await?;

    let review_url = format!("{}/dashboards/f/{}/{}", cfg.url, folder_uid, project_name);
    println!("--- --remote: review URL → {review_url}");
    Ok(())
}

/// Look up the project; create it if missing. Either way, register the
/// name + uuid in `hdx::` globals so subsequent calls in the existing
/// helpers route to the bundle-named project instead of the static
/// `bundle_verification`.
async fn ensure_project(bearer_token: &str, project_name: &str) -> Result<(), String> {
    let uuid = match hdx::find_project_uuid(bearer_token, project_name).await {
        Ok(u) => {
            println!("--- --remote: reusing existing project (uuid={})", u);
            u
        }
        Err(_) => {
            let u = hdx::create_project(bearer_token, project_name).await?;
            println!("--- --remote: created project (uuid={})", u);
            u
        }
    };
    hdx::set_guid_project(project_name.to_string(), uuid);
    Ok(())
}

async fn push_hydrolix(
    base: &str,
    bundle: &Bundle,
    bearer_token: &str,
    project_name: &str,
) -> Result<(), String> {
    hdx::shared_proj::check_dicts_and_funcs(bundle, project_name, base, bearer_token)
        .await
        .map_err(|e| format!("shared project validation failed: {e}"))?;

    for table in &bundle.tables {
        push_table(base, bearer_token, project_name, table).await?;
    }

    if bundle.summary_tables.is_some() && !bundle.tables.is_empty() {
        let secs = if *DELAY_MODE {
            println!("Waiting for tables to propagate to ClickHouse (--delay)...");
            TABLE_PROPAGATION_DELAY_SLOW_SECS
        } else {
            println!("Waiting for tables to propagate to ClickHouse...");
            TABLE_PROPAGATION_DELAY_SECS
        };
        sleep(Duration::from_secs(secs)).await;
    }

    if let Some(summaries) = &bundle.summary_tables {
        for summary in summaries {
            push_summary(base, bearer_token, project_name, summary).await?;
        }
    }

    Ok(())
}

async fn push_table(
    base: &str,
    bearer_token: &str,
    project_name: &str,
    table: &crate::models::bundle::Table,
) -> Result<(), String> {
    let table_uuid = match find_table_uuid_by_name(bearer_token, &table.name).await? {
        Some(uuid) => {
            println!("Reusing existing table '{}' (uuid={})", table.name, uuid);
            uuid
        }
        None => {
            println!("Creating table: {}", table.name);
            let uuid = hdx::table::create(bearer_token, &table.name)
                .await
                .map_err(|e| format!("create table {}: {e}", table.name))?;
            println!("  ✓ Table '{}' created (uuid={})", table.name, uuid);
            println!("Waiting for table to be ready...");
            sleep(Duration::from_secs(TABLE_READY_DELAY_SECS)).await;
            uuid
        }
    };

    for transform in &table.transforms {
        let transform_path = Path::new(base).join(&transform.path);
        let raw = tokio::fs::read_to_string(&transform_path)
            .await
            .map_err(|e| format!("read transform {}: {e}", transform_path.display()))?;
        let mut transform_json: Value = serde_json::from_str(&raw)
            .map_err(|e| format!("parse transform {}: {e}", transform_path.display()))?;

        substitute_transform_template_vars(&mut transform_json, project_name);

        match hdx::table::add_transform(bearer_token, &table_uuid, &transform_json).await {
            Ok(name) => println!("  ✓ Transform '{}' added", name),
            Err(e) => {
                // Most common rerun case: transform with this name already
                // exists on the table. Per Epic, --remote is "update in place"
                // and must not destroy reviewer-visible data — so we keep the
                // existing transform and continue rather than failing the push.
                eprintln!(
                    "  ⚠️  add_transform skipped for {} on {}: {}",
                    transform.path, table.name, e
                );
            }
        }

        if *DELAY_MODE {
            println!("Waiting for transform to propagate (--delay)...");
            sleep(Duration::from_secs(TRANSFORM_READY_DELAY_SECS)).await;
        }
    }

    Ok(())
}

async fn find_table_uuid_by_name(
    bearer_token: &str,
    table_name: &str,
) -> Result<Option<String>, String> {
    let body = hdx::table::get_list(bearer_token, false).await?;
    let json: Value =
        serde_json::from_str(&body).map_err(|e| format!("parse table list: {e} body={body}"))?;

    let empty_vec = vec![];
    let arr = if json.is_array() {
        json.as_array().unwrap_or(&empty_vec)
    } else if let Some(results) = json.get("results") {
        results.as_array().unwrap_or(&empty_vec)
    } else if let Some(data) = json.get("data") {
        data.as_array().unwrap_or(&empty_vec)
    } else {
        &empty_vec
    };

    for t in arr {
        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name == table_name {
            if let Some(uuid) = t.get("uuid").and_then(|v| v.as_str()) {
                return Ok(Some(uuid.to_string()));
            }
        }
    }
    Ok(None)
}

async fn push_summary(
    base: &str,
    bearer_token: &str,
    project_name: &str,
    summary: &crate::models::bundle::SummaryTable,
) -> Result<(), String> {
    let sql_path = Path::new(base).join(&summary.sql.path);
    let mut sql = tokio::fs::read_to_string(&sql_path)
        .await
        .map_err(|e| format!("read summary SQL {}: {e}", sql_path.display()))?;
    sql = sql
        .replace("__PROJECT_NAME__", project_name)
        .replace("__TABLE_NAME__", &summary.parent_table_name);

    println!(
        "Creating summary table: {} (parent: {}.{})",
        summary.name, project_name, summary.parent_table_name
    );

    hdx::table::exists(bearer_token, &summary.parent_table_name)
        .await
        .map_err(|e| format!("parent table {} not found: {e}", summary.parent_table_name))?;

    if find_table_uuid_by_name(bearer_token, &summary.name)
        .await?
        .is_some()
    {
        println!(
            "  ↺ Summary '{}' already exists, leaving in place",
            summary.name
        );
        return Ok(());
    }

    // ClickHouse may not have propagated the parent table yet even after the
    // API confirms it exists — retry up to ~3 minutes with 30s back-off.
    const SUMMARY_RETRIES: u32 = 6;
    const SUMMARY_RETRY_DELAY_SECS: u64 = 30;
    let mut last_err = String::new();
    for attempt in 1..=SUMMARY_RETRIES {
        match hdx::table::create_summary(bearer_token, &summary.name, &sql).await {
            Ok(_) => {
                println!("  ✓ Summary '{}' created", summary.name);
                return Ok(());
            }
            Err(e) => {
                last_err = e;
                if attempt < SUMMARY_RETRIES {
                    println!(
                        "  ↻ Summary '{}' not ready yet (attempt {}/{}), retrying in {}s...",
                        summary.name, attempt, SUMMARY_RETRIES, SUMMARY_RETRY_DELAY_SECS
                    );
                    sleep(Duration::from_secs(SUMMARY_RETRY_DELAY_SECS)).await;
                }
            }
        }
    }
    Err(format!("create summary {}: {last_err}", summary.name))
}

fn substitute_transform_template_vars(transform_json: &mut Value, project_name: &str) {
    let shared_project_name = hdx::shared_proj::get_name();
    if let Some(settings) = transform_json.get_mut("settings") {
        if let Some(sql_val) = settings.get("sql_transform").and_then(|v| v.as_str()) {
            let updated = sql_val
                .replace("__PROJECT_NAME__", project_name)
                .replace("__SHARED_PROJECT__", &shared_project_name);
            settings["sql_transform"] = Value::String(updated);
        }
    }
}

async fn push_grafana(
    base: &str,
    bundle: &Bundle,
    project_name: &str,
    cfg: &RemoteConfig,
) -> Result<String, String> {
    let folder_uid = ensure_subfolder(cfg, &cfg.bundling_folder_uid, project_name).await?;
    println!(
        "--- --remote: subfolder '{}' uid={}",
        project_name, folder_uid
    );

    let mut dashboard_paths: Vec<String> = vec![bundle.dashboard.path.clone()];
    if let Some(others) = &bundle.other_dashboards {
        for d in others {
            dashboard_paths.push(d.path.clone());
        }
    }

    for rel_path in dashboard_paths {
        let full_path = Path::new(base).join(&rel_path);
        let raw = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| format!("read dashboard {}: {e}", full_path.display()))?;

        let substituted =
            substitute_dashboard_placeholders(&raw, bundle, project_name, &rel_path, cfg);

        let mut dashboard: Value = serde_json::from_str(&substituted).map_err(|e| {
            format!(
                "parse dashboard {} after substitution: {e}",
                full_path.display()
            )
        })?;

        // Bundle dashboards live under a `{"dashboard": {...}}` wrapper. Grafana's
        // /api/dashboards/db expects the inner object as `dashboard` in the upsert
        // payload — without unwrapping we'd double-nest and Grafana sees an empty
        // outer title.
        if let Some(inner) = dashboard.get("dashboard").cloned() {
            if dashboard.as_object().is_some_and(|m| m.len() == 1) {
                dashboard = inner;
            }
        }

        rewrite_datasource_uid(&mut dashboard, &cfg.datasource_uid);

        // Strip server-assigned id so Grafana treats this as upsert-by-uid only.
        if let Some(map) = dashboard.as_object_mut() {
            map.remove("id");
        }

        let message = format!("review push: {} ({})", bundle.name, project_name);
        let url = upsert_dashboard(cfg, &folder_uid, &dashboard, &message).await?;
        println!("--- --remote: pushed {} → {}", rel_path, url);
    }

    Ok(folder_uid)
}

/// Apply the standard bundle-dashboard placeholder substitutions, qualifying
/// table and summary refs with the project prefix (required by the shared-
/// datasource review Grafana). Operates on the raw JSON text — same shape
/// as the existing `--local` path uses.
///
/// Order matters: table/summary vars are replaced before `__PROJECT_NAME__`
/// because many dashboard templates use the compound form
/// `__PROJECT_NAME__.__TABLE_NAME__`. Replacing `__PROJECT_NAME__` first and
/// then replacing `__TABLE_NAME__` with `project.table` would produce the
/// double-qualified `project.project.table`. Instead, we replace the compound
/// form in one pass, then clean up any remaining bare `__PROJECT_NAME__`
/// occurrences (titles, labels, etc.).
fn substitute_dashboard_placeholders(
    raw: &str,
    bundle: &Bundle,
    project_name: &str,
    rel_path: &str,
    cfg: &RemoteConfig,
) -> String {
    let mut out = raw.to_string();

    out = out.replace("__DATASOURCE__", &cfg.datasource_uid);
    out = out.replace(
        "__DASHBOARD_UUID__",
        &stable_dashboard_uid(project_name, rel_path),
    );
    out = out.replace("__SHARED_PROJECT__", &hdx::shared_proj::get_name());

    // Qualified table refs so queries resolve correctly through the shared
    // datasource (which is not pinned to any specific Hydrolix project).
    // Replace the prefixed form `__PROJECT_NAME__.__VAR__` first so we
    // never see `project.project.table`; then replace any bare `__VAR__`.
    for table in &bundle.tables {
        let qualified = format!("{}.{}", project_name, table.name);
        let prefixed = format!("__PROJECT_NAME__.{}", table.dashboard_var);
        out = out.replace(&prefixed, &qualified);
        out = out.replace(&table.dashboard_var, &qualified);
    }
    if let Some(summaries) = &bundle.summary_tables {
        for summary in summaries {
            let qualified = format!("{}.{}", project_name, summary.name);
            let prefixed = format!("__PROJECT_NAME__.{}", summary.dashboard_var);
            out = out.replace(&prefixed, &qualified);
            out = out.replace(&summary.dashboard_var, &qualified);
        }
    }

    // Replace any remaining __PROJECT_NAME__ (titles, labels, folder names).
    out = out.replace("__PROJECT_NAME__", project_name);

    out
}

/// Deterministic per-dashboard UID so `--remote` reruns update in place
/// (Grafana's `overwrite: true` only matches an existing dashboard via uid).
/// 40-char hex prefix of sha256(project + ":" + dashboard_path) — fits
/// Grafana's UID rules and is stable across re-runs.
fn stable_dashboard_uid(project_name: &str, rel_path: &str) -> String {
    let key = format!("{}:{}", project_name, rel_path);
    let digest = sha256::digest(key);
    digest.chars().take(40).collect()
}
