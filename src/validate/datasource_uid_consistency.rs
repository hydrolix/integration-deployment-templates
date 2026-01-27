// Validation: Check that dashboards use __DATASOURCE__ placeholder instead of hardcoded UIDs

use regex::Regex;
use tokio::fs;

use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let mut dashboard_path_list: Vec<String> = vec![];

    dashboard_path_list.push(format!("{}/{}", base, bundle.dashboard.path));

    if let Some(other_dashboards) = &bundle.other_dashboards {
        for other_dash in other_dashboards {
            dashboard_path_list.push(format!("{}/{}", base, other_dash.path));
        }
    }

    // Pattern to find datasource UIDs in JSON
    // Look for "uid": "something" in datasource contexts
    let uid_pattern = Regex::new(r#""datasource":\s*\{[^}]*"uid":\s*"([^"]+)""#).unwrap();

    for full_path in dashboard_path_list {
        let content = match fs::read_to_string(&full_path).await {
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

        // Find all datasource UIDs
        for cap in uid_pattern.captures_iter(&content) {
            if let Some(uid_match) = cap.get(1) {
                let uid = uid_match.as_str();

                // Check if it's NOT using the placeholder
                if uid != "__DATASOURCE__" {
                    // Allow special Grafana datasources
                    if uid == "-- Grafana --" || uid == "-- Mixed --" || uid == "-- Dashboard --" {
                        continue;
                    }

                    return Err(format!(
                        "ERROR: {}.{} Dashboard {} uses hardcoded datasource UID '{}' instead of '__DATASOURCE__' placeholder.\n  \
                         This will break deployment flexibility. All datasources should use '__DATASOURCE__' placeholder.\n",
                        file!(),
                        line!(),
                        dashboard_name,
                        uid
                    ));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uid_pattern_matching() {
        let uid_pattern = Regex::new(r#""datasource":\s*\{[^}]*"uid":\s*"([^"]+)""#).unwrap();

        let json1 = r#"{"datasource": {"type": "hydrolix", "uid": "__DATASOURCE__"}}"#;
        let caps: Vec<_> = uid_pattern.captures_iter(json1).collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].get(1).unwrap().as_str(), "__DATASOURCE__");

        let json2 = r#"{"datasource": {"type": "hydrolix", "uid": "abc123xyz"}}"#;
        let caps: Vec<_> = uid_pattern.captures_iter(json2).collect();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].get(1).unwrap().as_str(), "abc123xyz");
    }

    #[test]
    fn test_detects_hardcoded_uid() {
        let uid_pattern = Regex::new(r#""datasource":\s*\{[^}]*"uid":\s*"([^"]+)""#).unwrap();

        // Simulate dashboard content with hardcoded UID
        let dashboard_content = r#"{
            "panels": [
                {
                    "datasource": {
                        "type": "hydrolix-hydrolix-datasource",
                        "uid": "Pz8kLqC4z"
                    },
                    "title": "Test Panel"
                }
            ]
        }"#;

        let mut found_bad_uid = false;
        for cap in uid_pattern.captures_iter(dashboard_content) {
            if let Some(uid_match) = cap.get(1) {
                let uid = uid_match.as_str();
                if uid != "__DATASOURCE__"
                    && uid != "-- Grafana --"
                    && uid != "-- Mixed --"
                    && uid != "-- Dashboard --"
                {
                    found_bad_uid = true;
                    assert_eq!(uid, "Pz8kLqC4z");
                }
            }
        }
        assert!(found_bad_uid, "Should have detected hardcoded UID");
    }

    #[test]
    fn test_allows_special_grafana_datasources() {
        let uid_pattern = Regex::new(r#""datasource":\s*\{[^}]*"uid":\s*"([^"]+)""#).unwrap();

        let dashboard_content = r#"{
            "panels": [
                {
                    "datasource": {"type": "grafana", "uid": "-- Grafana --"}
                },
                {
                    "datasource": {"type": "mixed", "uid": "-- Mixed --"}
                },
                {
                    "datasource": {"type": "dashboard", "uid": "-- Dashboard --"}
                }
            ]
        }"#;

        for cap in uid_pattern.captures_iter(dashboard_content) {
            if let Some(uid_match) = cap.get(1) {
                let uid = uid_match.as_str();
                // These special UIDs should be allowed
                assert!(
                    uid == "-- Grafana --" || uid == "-- Mixed --" || uid == "-- Dashboard --",
                    "Special Grafana datasource UIDs should be allowed"
                );
            }
        }
    }
}
