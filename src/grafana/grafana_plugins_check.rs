// Validation: Check Grafana plugin requirements by querying deployed dashboards
// Uses Grafana's own API to detect which plugins are actually being used

use base64::Engine;
use serde_json::Value;
use std::collections::HashMap;

use crate::{GRAFANA_TOKEN, GRAFANA_USERNAME, GRAFANA_PASSWORD};
use crate::get_grafana_base_url;

#[derive(Debug, Clone)]
struct PluginInfo {
    id: String,
    name: String,
    plugin_type: String,
    #[allow(dead_code)]
    enabled: bool,
    #[allow(dead_code)]
    version: Option<String>,
}

#[derive(Debug, Clone)]
struct DashboardPluginUsage {
    #[allow(dead_code)]
    dashboard_uid: String,
    dashboard_title: String,
    #[allow(dead_code)]
    plugin_id: String,
    #[allow(dead_code)]
    plugin_type: String,
    panel_count: usize,
}

/// Check deployed dashboards for plugin usage after deployment
/// This should be called AFTER dashboards are created in Grafana
pub async fn check_deployed_dashboards(
    dashboard_uids: &[String],
    strict_plugins: bool,
) -> Result<(), String> {
    println!("\n🔌 Checking deployed dashboards for plugin usage...");

    if dashboard_uids.is_empty() {
        println!("  ℹ️  No dashboards to check");
        return Ok(());
    }

    // Get all installed plugins first
    let installed_plugins = get_installed_plugins().await?;
    let installed_map: HashMap<String, PluginInfo> = installed_plugins
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    // Track plugin usage across all dashboards
    let mut plugin_usage: HashMap<String, Vec<DashboardPluginUsage>> = HashMap::new();

    // Query each deployed dashboard
    for uid in dashboard_uids {
        match get_dashboard_by_uid(uid).await {
            Ok(dashboard) => {
                let plugins = extract_plugins_from_dashboard(&dashboard);

                for (plugin_id, count) in plugins {
                    plugin_usage
                        .entry(plugin_id.clone())
                        .or_insert_with(Vec::new)
                        .push(DashboardPluginUsage {
                            dashboard_uid: uid.clone(),
                            dashboard_title: dashboard
                                .get("dashboard")
                                .and_then(|d| d.get("title"))
                                .and_then(|t| t.as_str())
                                .unwrap_or(uid)
                                .to_string(),
                            plugin_id: plugin_id.clone(),
                            plugin_type: installed_map
                                .get(&plugin_id)
                                .map(|p| p.plugin_type.clone())
                                .unwrap_or_else(|| "unknown".to_string()),
                            panel_count: count,
                        });
                }
            }
            Err(e) => {
                eprintln!("  ⚠️  Could not check dashboard {}: {}", uid, e);
                // Continue checking other dashboards even if one fails
            }
        }
    }

    if plugin_usage.is_empty() {
        println!("  ✓ Dashboards only use built-in panels");
        return Ok(());
    }

    // Categorize plugins
    let mut installed_external: Vec<String> = Vec::new();
    let mut missing_plugins: Vec<String> = Vec::new();

    for plugin_id in plugin_usage.keys() {
        if let Some(plugin) = installed_map.get(plugin_id) {
            if plugin.plugin_type == "panel" && !is_builtin_plugin(&plugin.id) {
                installed_external.push(plugin_id.clone());
            }
        } else {
            missing_plugins.push(plugin_id.clone());
        }
    }

    // Report installed external plugins
    if !installed_external.is_empty() {
        println!(
            "\n✓ Using {} external plugin(s) (installed):",
            installed_external.len()
        );
        for plugin_id in &installed_external {
            let plugin = installed_map.get(plugin_id).unwrap();
            let usage = plugin_usage.get(plugin_id).unwrap();
            let total_panels: usize = usage.iter().map(|u| u.panel_count).sum();
            println!(
                "  • {} ({}) - {} panel(s) across {} dashboard(s)",
                plugin.name,
                plugin_id,
                total_panels,
                usage.len()
            );
        }
    }

    // Report missing plugins
    if !missing_plugins.is_empty() {
        let level = if strict_plugins { "ERROR" } else { "WARNING" };
        let icon = if strict_plugins { "❌" } else { "⚠️" };

        eprintln!("\n{} {}: Missing plugins detected!", icon, level);
        eprintln!("\nMissing plugins:");

        for plugin_id in &missing_plugins {
            let usage = plugin_usage.get(plugin_id).unwrap();
            let total_panels: usize = usage.iter().map(|u| u.panel_count).sum();

            eprintln!(
                "  • {} - {} panel(s) across {} dashboard(s)",
                plugin_id,
                total_panels,
                usage.len()
            );
            eprintln!("    Used in:");
            for u in usage {
                eprintln!(
                    "      - \"{}\" ({} panel(s))",
                    u.dashboard_title, u.panel_count
                );
            }
        }

        eprintln!("\n📋 To fix:");
        eprintln!("  1. Update grafana/container.ts to install missing plugins");
        eprintln!("  2. Add to GF_INSTALL_PLUGINS environment variable:");
        eprintln!(
            "     \"-e\", \"GF_INSTALL_PLUGINS={}\"",
            missing_plugins.join(",")
        );
        eprintln!("\n  Example:");
        eprintln!("     const cmd = new Deno.Command(\"docker\", {{");
        eprintln!("       args: [");
        eprintln!("         \"run\", \"--rm\", \"-d\", \"-p\", \"3000:3000\",");
        eprintln!(
            "         \"-e\", \"GF_INSTALL_PLUGINS={}\",",
            missing_plugins.join(",")
        );
        eprintln!("         \"javiani/grafana:latest\"");
        eprintln!("       ],");
        eprintln!("     }});");

        // FAIL HARD in strict mode
        if strict_plugins {
            return Err(format!(
                "Plugin validation failed: {} required plugin(s) missing. Please install: {}",
                missing_plugins.len(),
                missing_plugins.join(", ")
            ));
        } else {
            eprintln!("\n⚠️  Dashboards may not display correctly without these plugins");
        }
    }

    Ok(())
}

async fn get_dashboard_by_uid(uid: &str) -> Result<Value, String> {
    let url = format!("{}/api/dashboards/uid/{}", get_grafana_base_url(), uid);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Use token auth if available, otherwise fall back to basic auth
    if !GRAFANA_TOKEN.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
    } else {
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", *GRAFANA_USERNAME, *GRAFANA_PASSWORD));
        request = request.header("Authorization", format!("Basic {}", auth));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to fetch dashboard: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch dashboard: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse dashboard JSON: {}", e))
}

fn extract_plugins_from_dashboard(dashboard_data: &Value) -> HashMap<String, usize> {
    let mut plugin_counts: HashMap<String, usize> = HashMap::new();

    let dashboard = dashboard_data.get("dashboard").unwrap_or(dashboard_data);

    if let Some(panels) = dashboard.get("panels").and_then(|p| p.as_array()) {
        fn process_panel(panel: &Value, plugin_counts: &mut HashMap<String, usize>) {
            if let Some(panel_type) = panel.get("type").and_then(|t| t.as_str()) {
                *plugin_counts.entry(panel_type.to_string()).or_insert(0) += 1;
            }

            // Handle nested panels (rows)
            if let Some(nested_panels) = panel.get("panels").and_then(|p| p.as_array()) {
                for nested_panel in nested_panels {
                    process_panel(nested_panel, plugin_counts);
                }
            }
        }

        for panel in panels {
            process_panel(panel, &mut plugin_counts);
        }
    }

    // Filter out built-in panel types
    plugin_counts
        .into_iter()
        .filter(|(panel_type, _)| !is_builtin_panel_type(panel_type))
        .collect()
}

fn is_builtin_panel_type(panel_type: &str) -> bool {
    let builtin_types = [
        "graph",
        "timeseries",
        "stat",
        "gauge",
        "bargauge",
        "table",
        "text",
        "alertlist",
        "dashlist",
        "heatmap",
        "logs",
        "nodeGraph",
        "barchart",
        "candlestick",
        "canvas",
        "geomap",
        "histogram",
        "live",
        "news",
        "piechart",
        "state-timeline",
        "status-history",
        "table-old",
        "trace",
        "xychart",
        "row",
        "grafana-piechart-panel",
        "graph-old",
    ];

    builtin_types.contains(&panel_type)
}

fn is_builtin_plugin(plugin_id: &str) -> bool {
    // Grafana built-in plugins have specific prefixes
    plugin_id.starts_with("grafana-")
        && (plugin_id.ends_with("-datasource")
            || plugin_id.contains("builtin")
            || plugin_id == "grafana-clickhouse-datasource")
}

async fn get_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    let url = format!("{}/api/plugins", get_grafana_base_url());

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Use token auth if available, otherwise fall back to basic auth
    if !GRAFANA_TOKEN.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
    } else {
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", *GRAFANA_USERNAME, *GRAFANA_PASSWORD));
        request = request.header("Authorization", format!("Basic {}", auth));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to fetch plugins: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch plugins: {}", response.status()));
    }

    let plugins: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse plugins JSON: {}", e))?;

    let plugins_array = plugins
        .as_array()
        .ok_or("Unexpected plugins response format")?;

    Ok(plugins_array
        .iter()
        .map(|p| PluginInfo {
            id: p
                .get("id")
                .or_else(|| p.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            name: p
                .get("name")
                .or_else(|| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            plugin_type: p
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            enabled: p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            version: p
                .get("info")
                .and_then(|i| i.get("version"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
        .collect())
}
