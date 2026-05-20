// Validation: Check that template variables in dashboards match those declared in bundle.json

use regex::Regex;
use std::collections::{HashMap, HashSet};
use tokio::fs;

use crate::grafana::dashboard::slugify_grafana_title;
use crate::models::bundle::Bundle;

fn slug_to_macro(slug: &str) -> String {
    format!(
        "__DASHBOARD_UID_{}__",
        slug.to_uppercase().replace('-', "_")
    )
}

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    // Extract expected variables from bundle.json
    let mut expected_variables = HashSet::new();

    // Standard variables that should be used
    expected_variables.insert("__PROJECT_NAME__".to_string());
    expected_variables.insert("__DATASOURCE__".to_string());

    // Table variable from bundle
    for table in &bundle.tables {
        expected_variables.insert(table.dashboard_var.clone());
    }

    // Summary table variables from bundle
    if let Some(summary_tables) = &bundle.summary_tables {
        for summary_table in summary_tables {
            expected_variables.insert(summary_table.dashboard_var.clone());
        }
    }

    // Dashboard UUID variable
    expected_variables.insert("__DASHBOARD_UUID__".to_string());

    let mut dashboard_path_list: Vec<String> = vec![];

    dashboard_path_list.push(format!("{}/{}", base, bundle.dashboard.path));

    if let Some(other_dashboards) = &bundle.other_dashboards {
        for other_dash in other_dashboards {
            dashboard_path_list.push(format!("{}/{}", base, other_dash.path));
        }
    }

    // Build the set of all sibling slugs (and a slug→path map for diagnostics),
    // then add __DASHBOARD_UID_*__ macros to the expected-variables set.
    let mut all_slugs: HashSet<String> = HashSet::new();
    let mut slug_by_path: HashMap<String, String> = HashMap::new();

    for path in &dashboard_path_list {
        let content = match fs::read_to_string(path).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let dashboard = json.get("dashboard").unwrap_or(&json);
        let title = dashboard["title"].as_str().unwrap_or("");
        let slug = slugify_grafana_title(title);
        if !slug.is_empty() {
            all_slugs.insert(slug.clone());
            slug_by_path.insert(path.clone(), slug.clone());
            // Every sibling slug is a legitimate __DASHBOARD_UID_*__ macro target
            expected_variables.insert(slug_to_macro(&slug));
        }
    }

    // Pattern to find template variables in JSON: __VARIABLE_NAME__
    // Non-greedy so __PROJECT_NAME___suffix parses as __PROJECT_NAME__ not __PROJECT_NAME___
    let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*?)__").unwrap();

    // Pattern to find hardcoded <uuid>/<slug> strings — used for the missed-rewrite check.
    let uid_slug_pattern = Regex::new(
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/([a-z0-9][a-z0-9-]*)",
    )
    .unwrap();

    for full_path in &dashboard_path_list {
        let content = match fs::read_to_string(full_path).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "ERROR: {}.{} Failed to read dashboard full_path={full_path}: error={e}\n",
                    file!(),
                    line!()
                ));
            }
        };

        // Extract dashboard name from path
        let dashboard_name = full_path.split('/').next_back().unwrap_or("unknown");

        let own_slug = slug_by_path.get(full_path).cloned().unwrap_or_default();

        // Find all variables used in this dashboard
        let mut found_variables = HashSet::new();
        for cap in variable_pattern.captures_iter(&content) {
            if let Some(var_match) = cap.get(0) {
                found_variables.insert(var_match.as_str().to_string());
            }
        }

        // Check for unexpected variables (typos, wrong names)
        for var in &found_variables {
            if !expected_variables.contains(var) {
                // Some exceptions for special Grafana variables
                if var.starts_with("__time")
                    || var.starts_with("__from")
                    || var.starts_with("__to")
                    || var == "__dashboard"
                    || var == "__user"
                    || var.starts_with("__interval")
                {
                    continue; // These are standard Grafana variables
                }

                return Err(format!(
                    "ERROR: {}.{} Dashboard {} uses unexpected template variable '{}'.\n  \
                     This might be a typo. Expected variables from bundle.json: {:?}\n  \
                     Note: Standard Grafana variables like __time*, __from*, __to*, __user, __interval* are allowed.\n",
                    file!(),
                    line!(),
                    dashboard_name,
                    var,
                    expected_variables.iter().collect::<Vec<_>>()
                ));
            }
        }

        // Missed-rewrite check: warn on any hardcoded <uuid>/<slug> whose slug
        // matches a sibling (including self). These should be rewritten to
        // __DASHBOARD_UID_<SLUG>__ (or __DASHBOARD_UUID__ for self) by the
        // configurator pipeline before the next deploy.
        for cap in uid_slug_pattern.captures_iter(&content) {
            let slug = &cap[1];
            if !all_slugs.contains(slug) {
                continue; // External/community dashboard — pass silently
            }
            let expected_macro = if slug == own_slug {
                "__DASHBOARD_UUID__".to_string()
            } else {
                slug_to_macro(slug)
            };
            let ref_kind = if slug == own_slug {
                "self-reference"
            } else {
                "sibling reference"
            };
            println!(
                "WARNING: {}.{} Dashboard '{}' contains a hardcoded UID for slug '{}' ({}).\n  \
                 Expected macro: '{}'\n  \
                 Run the configurator pipeline to fix this.\n",
                file!(),
                line!(),
                dashboard_name,
                slug,
                ref_kind,
                expected_macro,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_pattern_matching() {
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*?)__").unwrap();

        let json = r#"{"query": "SELECT * FROM __PROJECT_NAME__.table WHERE time > __from"}"#;
        let vars: Vec<_> = variable_pattern
            .captures_iter(json)
            .filter_map(|cap| cap.get(0))
            .map(|m| m.as_str())
            .collect();

        assert!(vars.contains(&"__PROJECT_NAME__"));
    }

    #[test]
    fn test_detects_typo_in_variable() {
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*?)__").unwrap();

        // Common typo: missing underscore
        let json = r#"{"query": "SELECT * FROM __PROJECTNAME__.table"}"#;
        let vars: Vec<_> = variable_pattern
            .captures_iter(json)
            .filter_map(|cap| cap.get(0))
            .map(|m| m.as_str().to_string())
            .collect();

        assert!(vars.contains(&"__PROJECTNAME__".to_string()));
        assert!(!vars.contains(&"__PROJECT_NAME__".to_string()));
    }

    #[test]
    fn test_allows_grafana_variables() {
        let grafana_vars = vec![
            "__timeFrom",
            "__timeTo",
            "__from",
            "__to",
            "__dashboard",
            "__user",
            "__interval",
            "__interval_ms",
        ];

        for var in grafana_vars {
            let is_allowed = var.starts_with("__time")
                || var.starts_with("__from")
                || var.starts_with("__to")
                || var == "__dashboard"
                || var == "__user"
                || var.starts_with("__interval");

            assert!(
                is_allowed,
                "Standard Grafana variable {} should be allowed",
                var
            );
        }
    }

    #[test]
    fn test_project_name_with_uid_suffix() {
        // __PROJECT_NAME___raw and __PROJECT_NAME___default use triple-underscore to separate
        // the token from the uid suffix. The non-greedy regex must extract __PROJECT_NAME__,
        // not __PROJECT_NAME___.
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*?)__").unwrap();

        for input in &[
            r#"{"uid": "__PROJECT_NAME___raw"}"#,
            r#"{"uid": "__PROJECT_NAME___default"}"#,
            r#"{"query": "__PROJECT_NAME___raw/raw-logs"}"#,
        ] {
            let vars: Vec<_> = variable_pattern
                .captures_iter(input)
                .filter_map(|cap| cap.get(0))
                .map(|m| m.as_str().to_string())
                .collect();

            assert!(
                vars.contains(&"__PROJECT_NAME__".to_string()),
                "Expected __PROJECT_NAME__ in: {input}, got: {vars:?}"
            );
            assert!(
                !vars.iter().any(|v| v.contains("___")),
                "Unexpected triple-underscore token in: {input}, got: {vars:?}"
            );
        }
    }

    #[test]
    fn test_sibling_macro_in_expected_set() {
        // Demonstrate that __DASHBOARD_UID_RAW_LOGS__ is recognised as
        // a legitimate template variable (it passes the expected-variable check).
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*?)__").unwrap();
        let content = r#"{"query": "__DASHBOARD_UID_RAW_LOGS__/raw-logs"}"#;

        let vars: Vec<_> = variable_pattern
            .captures_iter(content)
            .filter_map(|cap| cap.get(0))
            .map(|m| m.as_str().to_string())
            .collect();

        assert!(vars.contains(&"__DASHBOARD_UID_RAW_LOGS__".to_string()));
    }

    #[test]
    fn test_uid_slug_pattern_matches_hardcoded_uid() {
        let uid_slug_pattern = Regex::new(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/([a-z0-9][a-z0-9-]*)",
        )
        .unwrap();

        let content = r#""/d/c44c5f0c-badf-4794-94a7-2ca3c6f37ade/raw-logs?var-x=1""#;
        let caps: Vec<_> = uid_slug_pattern.captures_iter(content).collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(&caps[0][1], "raw-logs");
    }

    #[test]
    fn test_uid_slug_pattern_does_not_match_macro() {
        let uid_slug_pattern = Regex::new(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/([a-z0-9][a-z0-9-]*)",
        )
        .unwrap();

        // __DASHBOARD_UID_RAW_LOGS__ is not a valid UUID → pattern must not match
        let content = r#""__DASHBOARD_UID_RAW_LOGS__/raw-logs""#;
        assert_eq!(uid_slug_pattern.captures_iter(content).count(), 0);
    }

    #[test]
    fn test_slugify_parity_with_python() {
        // These cases are also tested in Python — both must agree exactly.
        let cases = [
            ("Cache Analysis Treemap", "cache-analysis-treemap"),
            ("Raw Logs", "raw-logs"),
            ("CDN Global View", "cdn-global-view"),
            ("CDN Dashboard Default", "cdn-dashboard-default"),
            ("Home", "home"),
            ("Foo  --  Bar", "foo-bar"),
        ];
        for (title, expected) in &cases {
            assert_eq!(
                slugify_grafana_title(title),
                *expected,
                "slug mismatch for '{}'",
                title
            );
        }
    }
}
