// Validation: Check that dashboard JSON is valid and contains required template variables

import type { Bundle } from "../types/bundle.ts";
import { getErrorMessage } from "../utils/error.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  const dashboardPathList: string[] = [];
  
  dashboardPathList.push(`${base}/${bundle.dashboard.path}`);
  
  if (bundle.other_dashboards) {
    for (const otherDash of bundle.other_dashboards) {
      dashboardPathList.push(`${base}/${otherDash.path}`);
    }
  }
  
  for (const fullPath of dashboardPathList) {
    const content = await Deno.readTextFile(fullPath);
    
    // Build list of required template variables
    const mustHave: string[] = [
      "__DASHBOARD_UUID__",
      "__DATASOURCE__",
      "__PROJECT_NAME__",
    ];
    
    // Add all table dashboard_vars
    for (const table of bundle.tables) {
      mustHave.push(table.dashboard_var);
    }
    
    // Check that all required variables are present
    for (const variable of mustHave) {
      if (!content.includes(variable)) {
        throw new Error(
          `Dashboard must have ${variable} full_path=${fullPath}`
        );
      }
    }
    
    // Parse and validate JSON structure
    let dashboardJson;
    try {
      dashboardJson = JSON.parse(content);
    } catch (e) {
      throw new Error(
        `Invalid JSON full_path=${fullPath} error=${getErrorMessage(e)}`
      );
    }
    
    // Check that top element is "dashboard"
    if (!dashboardJson.dashboard || typeof dashboardJson.dashboard !== 'object') {
      throw new Error(
        `Invalid dashboard - top element must be dashboard. full_path=${fullPath}`
      );
    }
    
    // Check that id is not set
    if (dashboardJson.id !== undefined && dashboardJson.id !== null) {
      throw new Error(
        `Invalid dashboard - cannot have Id set. full_path=${fullPath}`
      );
    }
  }
}