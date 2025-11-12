// Validate alert rules file structure

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;

use crate::bundle_struct::Bundle;

#[derive(Debug, Serialize, Deserialize)]
struct AlertRulesFile {
    #[serde(rename = "apiVersion")]
    api_version: Option<i32>,
    groups: Vec<AlertGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AlertGroup {
    #[serde(rename = "orgId")]
    org_id: Option<i32>,
    name: String,
    folder: String,
    interval: String,
    rules: Vec<AlertRule>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AlertRule {
    uid: String,
    title: String,
    condition: String,
    data: Vec<Value>,
}

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    if bundle.alert_rules.is_none() {
        println!("ℹ️  No alert rules defined in bundle (optional)");
        return Ok(());
    }

    let alert_rules_ref = bundle.alert_rules.as_ref().unwrap();
    let alert_rules_path = format!("{}/{}", base, alert_rules_ref.path);

    println!("Validating alert rules at {}...", alert_rules_path);

    let content = fs::read_to_string(&alert_rules_path)
        .await
        .map_err(|e| {
            format!(
                "Failed to read alert rules file at {}: {}",
                alert_rules_path, e
            )
        })?;

    let alert_rules: AlertRulesFile = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse alert rules JSON at {}: {}",
            alert_rules_path, e
        )
    })?;

    // Validate structure
    if alert_rules.api_version.is_none() {
        return Err("alert_rules.apiVersion is required".to_string());
    }

    if alert_rules.groups.is_empty() {
        return Err("alert_rules.groups must contain at least one group".to_string());
    }

    // Validate each group
    for (i, group) in alert_rules.groups.iter().enumerate() {
        if group.name.is_empty() {
            return Err(format!("Group {}: name is required", i));
        }

        if group.folder.is_empty() {
            return Err(format!("Group {} ({}): folder is required", i, group.name));
        }

        if group.interval.is_empty() {
            return Err(format!("Group {} ({}): interval is required", i, group.name));
        }

        if group.rules.is_empty() {
            return Err(format!(
                "Group {} ({}): rules must contain at least one rule",
                i, group.name
            ));
        }

        // Validate each rule
        for (j, rule) in group.rules.iter().enumerate() {
            if rule.uid.is_empty() {
                return Err(format!("Group {}, Rule {}: uid is required", group.name, j));
            }

            if rule.title.is_empty() {
                return Err(format!(
                    "Group {}, Rule {}: title is required",
                    group.name, j
                ));
            }

            if rule.condition.is_empty() {
                return Err(format!(
                    "Group {}, Rule {}: condition is required",
                    group.name, j
                ));
            }
        }
    }

    let total_rules = get_total_rules(&alert_rules);
    println!(
        "✓ Alert rules file is valid ({} group(s), {} rule(s))",
        alert_rules.groups.len(),
        total_rules
    );

    Ok(())
}

fn get_total_rules(alert_rules: &AlertRulesFile) -> usize {
    alert_rules
        .groups
        .iter()
        .map(|group| group.rules.len())
        .sum()
}
