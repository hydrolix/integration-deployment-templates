// Dashboard-only deployment (no table/data creation) with alert rules support
// UPDATED: Returns array of all dashboard UIDs for plugin validation

import type { Bundle, Output } from "./types/bundle.ts";
import * as grafana from "./grafana/interface.ts";
import * as hdx from "./hdx.ts";
import * as hdxShared from "./hdx_shared.ts";
import { GRAFANA_LOCATION } from "./grafana/container.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";

export async function run(
  base: string,
  bundle: Bundle,
  output: Output
): Promise<string[]> {  // ← CHANGED: Returns array instead of string
  const projectName = hdx.createProjectName();
  const sharedProjectName = hdxShared.getSharedProjectName();
  
  // Create datalink
  const datalink = await grafana.createDatalink(projectName);
  
  output.cluster_domain = BUNDLE_TESTING_CLUSTER;
  output.project_name = projectName;
  output.grafana_domain = `${GRAFANA_LOCATION}/`;
  output.datalink = datalink;
  
  // Load and process dashboard template
  let dashboardData = await loadDashboardTemplate(base, bundle, projectName, datalink, sharedProjectName);
  
  // Replace table variables with table names
  for (const table of bundle.tables) {
    const fullTableName = `${projectName}.${table.name}`;
    dashboardData = dashboardData.replace(table.dashboard_var, fullTableName);
  }
  
  // Replace summary table variables if present
  if (bundle.summary_tables) {
    for (const summary of bundle.summary_tables) {
      const fullTableName = `${projectName}.${summary.name}`;
      dashboardData = dashboardData.replace(summary.dashboard_var, fullTableName);
    }
  }
  
  // Create Grafana dashboard
  const dashboardId = await grafana.createDashboard(dashboardData);
  
  output.dashboard_id = dashboardId;
  
  // Collect all dashboard UIDs
  const allDashboardUids: string[] = [dashboardId];  // ← ADDED: Array to collect UIDs
  
  // Create other dashboards if present
  if (bundle.other_dashboards) {
    for (const otherDash of bundle.other_dashboards) {
      console.log(`Creating additional dashboard: ${otherDash.path}`);

      let otherDashboardData = await Deno.readTextFile(`${base}/${otherDash.path}`);

      // Replace template variables
      otherDashboardData = otherDashboardData.replace(/__PROJECT_NAME__/g, projectName);
      otherDashboardData = otherDashboardData.replace(/__DATASOURCE__/g, datalink);
      otherDashboardData = otherDashboardData.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());
      otherDashboardData = otherDashboardData.replace(/__SHARED_PROJECT__/g, sharedProjectName);

      // Replace VAR_SUMMARY variables
      if (bundle.summary_tables && bundle.summary_tables.length > 0) {
        otherDashboardData = otherDashboardData.replace(/\$\{?VAR_SUMMARY_MIN\}?/g, `${projectName}.${bundle.summary_tables[0].name}`);
      }
      if (bundle.summary_tables && bundle.summary_tables.length > 1) {
        otherDashboardData = otherDashboardData.replace(/\$\{?VAR_SUMMARY_HOUR\}?/g, `${projectName}.${bundle.summary_tables[1].name}`);
      }

      // Replace table variables
      for (const table of bundle.tables) {
        otherDashboardData = otherDashboardData.replace(new RegExp(table.dashboard_var, 'g'), table.name);
      }

      // Replace summary table variables
      if (bundle.summary_tables) {
        for (const summary of bundle.summary_tables) {
          otherDashboardData = otherDashboardData.replace(new RegExp(summary.dashboard_var, 'g'), summary.name);
        }
      }

      const otherDashUid = await grafana.createDashboard(otherDashboardData);  // ← CHANGED: Capture UID
      allDashboardUids.push(otherDashUid);  // ← ADDED: Add to collection
      console.log(`✓ Created dashboard: ${otherDash.path} (UID: ${otherDashUid})`);
    }
  }
  
  // Create alert rules if present
  if (bundle.alert_rules) {
    await createAlertRules(base, bundle, projectName, datalink, dashboardId, sharedProjectName);
  }
  
  return allDashboardUids;  // ← CHANGED: Return array instead of single UID
}

async function loadDashboardTemplate(
  base: string,
  bundle: Bundle,
  projectName: string,
  datalink: string,
  sharedProjectName: string
): Promise<string> {
  const path = `${base}/${bundle.dashboard.path}`;
  
  let dashboard = await Deno.readTextFile(path);
  
  dashboard = dashboard.replace(/__PROJECT_NAME__/g, projectName);
  dashboard = dashboard.replace(/__DATASOURCE__/g, datalink);
  dashboard = dashboard.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());
  dashboard = dashboard.replace(/__SHARED_PROJECT__/g, sharedProjectName);
  
  return dashboard;
}

async function createAlertRules(
  base: string,
  bundle: Bundle,
  projectName: string,
  datalink: string,
  dashboardId: string,
  sharedProjectName: string
): Promise<void> {
  if (!bundle.alert_rules) {
    return;
  }
  
  console.log("Loading and processing alert rules...");
  
  const alertRulesPath = `${base}/${bundle.alert_rules.path}`;
  let alertRulesContent = await Deno.readTextFile(alertRulesPath);
  
  // Replace template variables
  alertRulesContent = alertRulesContent.replace(/__PROJECT_NAME__/g, projectName);
  alertRulesContent = alertRulesContent.replace(/__DATASOURCE__/g, datalink);
  alertRulesContent = alertRulesContent.replace(/__DASHBOARD_UUID__/g, dashboardId);
  alertRulesContent = alertRulesContent.replace(/__SHARED_PROJECT__/g, sharedProjectName);
  
  // Replace table variables with full table names
  for (const table of bundle.tables) {
    const fullTableName = `${projectName}.${table.name}`;
    alertRulesContent = alertRulesContent.replace(new RegExp(table.dashboard_var, 'g'), fullTableName);
  }
  
  // Replace summary table variables if present
  if (bundle.summary_tables) {
    for (const summary of bundle.summary_tables) {
      const fullTableName = `${projectName}.${summary.name}`;
      alertRulesContent = alertRulesContent.replace(new RegExp(summary.dashboard_var, 'g'), fullTableName);
    }
  }
  
  await grafana.createAlertRules(alertRulesContent);
}