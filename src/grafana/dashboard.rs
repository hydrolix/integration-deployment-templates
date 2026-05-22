use std::collections::HashMap;

use bundle_validator::hdx;
use serde_json::Value;
use tokio::fs;

use crate::grafana::http;
use crate::models::bundle::Bundle;
use crate::GRAFANA_LOCATION;

/// Grafana-style title slugification: lowercase, any run of non-ASCII-alphanumeric
/// characters collapses to a single hyphen, leading/trailing hyphens stripped.
///
/// Mirrors `slugify_grafana_title` in `scripts/utils/file_utils.py` exactly —
/// cross-language parity is enforced by unit tests in both languages.
pub fn slugify_grafana_title(title: &str) -> String {
    let lower = title.to_lowercase();
    let parts: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    parts.join("-")
}

/// Convert a slug to its `__DASHBOARD_UID_*__` macro name.
///
/// Example: "raw-logs" -> "__DASHBOARD_UID_RAW_LOGS__"
fn slug_to_macro(slug: &str) -> String {
    format!(
        "__DASHBOARD_UID_{}__",
        slug.to_uppercase().replace('-', "_")
    )
}

/// Build a map of {slug -> freshly-generated UUID} for every dashboard in the
/// bundle (primary + others).  All UUIDs are generated in one pass BEFORE any
/// per-dashboard substitution runs, so sibling references are consistent.
pub async fn build_sibling_uid_map(
    base: &str,
    bundle: &Bundle,
) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();

    // Primary dashboard
    let primary_path = format!("{}/{}", base, bundle.dashboard.path);
    if let Err(e) = add_dashboard_to_map(&primary_path, &mut map).await {
        return Err(e);
    }

    // Other dashboards
    if let Some(others) = &bundle.other_dashboards {
        for d in others {
            let path = format!("{}/{}", base, d.path);
            if let Err(e) = add_dashboard_to_map(&path, &mut map).await {
                return Err(e);
            }
        }
    }

    Ok(map)
}

async fn add_dashboard_to_map(path: &str, map: &mut HashMap<String, String>) -> Result<(), String> {
    let content = match fs::read_to_string(path).await {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to read dashboard {}: {}", path, e)),
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return Err(format!("Failed to parse dashboard JSON {}: {}", path, e)),
    };

    let dashboard = json.get("dashboard").unwrap_or(&json);
    let title = dashboard["title"].as_str().unwrap_or("");
    let slug = slugify_grafana_title(title);

    if !slug.is_empty() {
        // Derive a stable, project-scoped UID from the dashboard filename stem.
        // "raw.json" → "__PROJECT_NAME___raw", "Raw Logs.json" → "__PROJECT_NAME___raw_logs"
        // __PROJECT_NAME__ is resolved later by the standard substitution pass.
        let stem = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&slug)
            .to_lowercase()
            .replace(' ', "_");
        map.insert(slug, format!("__PROJECT_NAME___{}", stem));
    }

    Ok(())
}

/// Replace every `__DASHBOARD_UID_<SLUG>__` occurrence in `contents` with the
/// corresponding project-scoped UID from `sibling_uid_map`.
///
/// UID values are of the form `__PROJECT_NAME___<stem>` and still contain the
/// `__PROJECT_NAME__` placeholder, which the caller resolves in a subsequent
/// substitution pass.  Patterns already using `__PROJECT_NAME___<stem>/<slug>`
/// are left untouched — they are resolved naturally when `__PROJECT_NAME__` is
/// replaced.
pub fn apply_sibling_uid_subs(contents: &mut String, sibling_uid_map: &HashMap<String, String>) {
    for (slug, uid) in sibling_uid_map {
        let macro_name = slug_to_macro(slug);
        *contents = contents.replace(&macro_name, uid);
    }
}

/// Look up this dashboard's own UID from the sibling map using its title slug.
/// Falls back to a `__PROJECT_NAME___<slug>` pattern if the title is missing or
/// not in the map, keeping the UID in the same project-scoped form.
pub(crate) fn own_uid_from_map(content: &str, sibling_uid_map: &HashMap<String, String>) -> String {
    let json: Value = serde_json::from_str(content).unwrap_or(Value::Null);
    let dashboard = json.get("dashboard").unwrap_or(&json);
    let title = dashboard["title"].as_str().unwrap_or("");
    let slug = slugify_grafana_title(title);
    sibling_uid_map
        .get(&slug)
        .cloned()
        .unwrap_or_else(|| format!("__PROJECT_NAME___{}", slug.replace('-', "_")))
}

pub async fn create(dashboard_data: &str) -> Result<String, String> {
    let url = format!("http://{GRAFANA_LOCATION}/api/dashboards/import");

    let boxed_str: Box<str> = dashboard_data.into();

    let result_data = match http::post_string_basic_auth(
        &url,
        "admin",
        "admin",
        boxed_str.clone(),
        Some(vec![("X-Grafana-Org-Id", "1")]), // Add the X-Grafana-Org-Id header
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("ERROR: {}.{} {e}", file!(), line!()));
        }
    };

    let result_json: Value = match serde_json::from_str(&result_data) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{}  {e}", file!(), line!())),
    };

    // Access the "uid"
    if let Some(uid) = result_json["uid"].as_str() {
        return Ok(uid.to_string());
    };

    Err(format!(
        "ERROR: No UID in the dashboard {}.{}",
        file!(),
        line!()
    ))
}

pub async fn create_others(
    bundle: &Bundle,
    project_name: &str,
    base: &str,
    datalink: &str,
    sibling_uid_map: &HashMap<String, String>,
) -> Result<Vec<String>, String> {
    let mut all_dashboard_uuids = vec![];
    // Create other dashboards if present
    let dashboards = match &bundle.other_dashboards {
        Some(v) => v,
        None => return Ok(all_dashboard_uuids),
    };
    let shared_project_name = hdx::shared_proj::get_name();

    for d in dashboards {
        println!("Creating additional dashboard: {}", d.path);

        let full_path = format!("{}/{}", base, d.path);
        let mut contents = match fs::read_to_string(&full_path).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "Failed to read other dashboard {}: {}",
                    full_path, e
                ))
            }
        };

        // Resolve this dashboard's own UID from the pre-built map
        let own_uid = own_uid_from_map(&contents, sibling_uid_map);

        // Apply sibling UID substitutions before __DASHBOARD_UUID__ replacement.
        // __DASHBOARD_UUID__ must be replaced before __PROJECT_NAME__ so that
        // __PROJECT_NAME__ inside the UID value (e.g. __PROJECT_NAME___raw) gets resolved.
        apply_sibling_uid_subs(&mut contents, sibling_uid_map);
        contents = contents.replace("__DASHBOARD_UUID__", &own_uid);

        contents = contents.replace("__PROJECT_NAME__", project_name);
        contents = contents.replace("__DATASOURCE__", datalink);
        contents = contents.replace("__SHARED_PROJECT__", &shared_project_name);

        // Replace summary table variables if present
        if let Some(summary_tables) = &bundle.summary_tables {
            if !summary_tables.is_empty() {
                let summary_min = format!("{}.{}", project_name, summary_tables[0].name);
                contents = contents
                    .replace("${VAR_SUMMARY_MIN}", &summary_min)
                    .replace("$VAR_SUMMARY_MIN", &summary_min);
            }
            if summary_tables.len() > 1 {
                let summary_hour = format!("{}.{}", project_name, summary_tables[1].name);
                contents = contents
                    .replace("${VAR_SUMMARY_HOUR}", &summary_hour)
                    .replace("$VAR_SUMMARY_HOUR", &summary_hour);
            }
        }

        // Replace table dashboard vars
        for table in &bundle.tables {
            contents = contents.replace(&table.dashboard_var, &table.name);
        }

        // Replace summary table dashboard vars
        if let Some(summary_tables) = &bundle.summary_tables {
            for summary in summary_tables {
                contents = contents.replace(&summary.dashboard_var, &summary.name);
            }
        }

        let other_dash_uid = match create(&contents).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to create other dashboard: {}", e)),
        };

        all_dashboard_uuids.push(other_dash_uid.clone());
        println!("✓ Created dashboard: {} (UID: {})", d.path, other_dash_uid);
    }

    Ok(all_dashboard_uuids)
}

pub async fn load_template(
    base: &str,
    bundle: &Bundle,
    project_name: &str,
    datalink: &str,
    sibling_uid_map: &HashMap<String, String>,
) -> Result<String, String> {
    let path = format!("{base}/{}", bundle.dashboard.path);

    let mut dashboard = match fs::read_to_string(&path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read dashboard path={path}: {e}",
                file!(),
                line!()
            ));
        }
    };

    // Resolve own UID from the pre-built map
    let own_uid = own_uid_from_map(&dashboard, sibling_uid_map);

    // Apply sibling UID substitutions before __DASHBOARD_UUID__ replacement.
    // __DASHBOARD_UUID__ must be replaced before __PROJECT_NAME__ so that
    // __PROJECT_NAME__ inside the UID value (e.g. __PROJECT_NAME___raw) gets resolved.
    apply_sibling_uid_subs(&mut dashboard, sibling_uid_map);
    dashboard = dashboard.replace("__DASHBOARD_UUID__", &own_uid);

    dashboard = dashboard.replace("__PROJECT_NAME__", project_name);
    dashboard = dashboard.replace("__DATASOURCE__", datalink);

    Ok(dashboard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_grafana_title_ascii() {
        assert_eq!(slugify_grafana_title("Raw Logs"), "raw-logs");
        assert_eq!(
            slugify_grafana_title("CDN Dashboard Default"),
            "cdn-dashboard-default"
        );
        assert_eq!(
            slugify_grafana_title("Cache Analysis Treemap"),
            "cache-analysis-treemap"
        );
        assert_eq!(slugify_grafana_title("CDN Global View"), "cdn-global-view");
        assert_eq!(slugify_grafana_title("Home"), "home");
    }

    #[test]
    fn test_slugify_grafana_title_punctuation() {
        // Multiple separators collapse to a single hyphen
        assert_eq!(slugify_grafana_title("Foo  --  Bar"), "foo-bar");
    }

    #[test]
    fn test_slugify_grafana_title_accented() {
        // Non-ASCII chars are treated as separators
        assert_eq!(slugify_grafana_title("Café Bar"), "caf-bar");
    }

    #[test]
    fn test_slug_to_macro() {
        assert_eq!(slug_to_macro("raw-logs"), "__DASHBOARD_UID_RAW_LOGS__");
        assert_eq!(
            slug_to_macro("cdn-dashboard-default"),
            "__DASHBOARD_UID_CDN_DASHBOARD_DEFAULT__"
        );
    }

    #[test]
    fn test_apply_sibling_uid_subs() {
        let mut content =
            "uid: __DASHBOARD_UID_RAW_LOGS__ and __DASHBOARD_UID_CDN_DASHBOARD_DEFAULT__"
                .to_string();
        let mut map = HashMap::new();
        map.insert(
            "raw-logs".to_string(),
            "__PROJECT_NAME___raw".to_string(),
        );
        map.insert(
            "cdn-dashboard-default".to_string(),
            "__PROJECT_NAME___default".to_string(),
        );

        apply_sibling_uid_subs(&mut content, &map);

        assert_eq!(
            content,
            "uid: __PROJECT_NAME___raw and __PROJECT_NAME___default"
        );
    }

    #[test]
    fn test_apply_sibling_uid_subs_leaves_project_name_urls_intact() {
        // __PROJECT_NAME___raw/raw-logs must NOT be rewritten — the caller's
        // subsequent __PROJECT_NAME__ → project_name substitution handles it.
        let mut map = HashMap::new();
        map.insert(
            "raw-logs".to_string(),
            "__PROJECT_NAME___raw".to_string(),
        );
        map.insert(
            "cdn-dashboard-default".to_string(),
            "__PROJECT_NAME___default".to_string(),
        );

        let mut content = "\"query\": \"__PROJECT_NAME___raw/raw-logs\"".to_string();
        apply_sibling_uid_subs(&mut content, &map);
        assert_eq!(content, "\"query\": \"__PROJECT_NAME___raw/raw-logs\"");

        let mut content2 =
            "/d/__PROJECT_NAME___default/cdn-dashboard-default?foo=bar".to_string();
        apply_sibling_uid_subs(&mut content2, &map);
        assert_eq!(
            content2,
            "/d/__PROJECT_NAME___default/cdn-dashboard-default?foo=bar"
        );
    }
}
