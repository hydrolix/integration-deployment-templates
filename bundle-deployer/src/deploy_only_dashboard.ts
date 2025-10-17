// Dashboard-only deployment (no table/data creation)

import type { Bundle, Output } from "./types/bundle.ts";
import * as grafana from "./grafana/interface.ts";
import * as hdx from "./hdx.ts";
import { GRAFANA_LOCATION } from "./grafana/container.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";

export async function run(
  base: string,
  bundle: Bundle,
  output: Output
): Promise<string> {
  const projectName = hdx.createProjectName();
  
  // Create datalink
  const datalink = await grafana.createDatalink(projectName);
  
  output.cluster_domain = BUNDLE_TESTING_CLUSTER;
  output.project_name = projectName;
  output.grafana_domain = `${GRAFANA_LOCATION}/`;
  output.datalink = datalink;
  
  // Load and process dashboard template
  let dashboardData = await loadDashboardTemplate(base, bundle, projectName, datalink);
  
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
  return dashboardId;
}

async function loadDashboardTemplate(
  base: string,
  bundle: Bundle,
  projectName: string,
  datalink: string
): Promise<string> {
  const path = `${base}/${bundle.dashboard.path}`;
  
  let dashboard = await Deno.readTextFile(path);
  
  dashboard = dashboard.replace(/__PROJECT_NAME__/g, projectName);
  dashboard = dashboard.replace(/__DATASOURCE__/g, datalink);
  dashboard = dashboard.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());
  
  return dashboard;
}