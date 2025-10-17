// Validation: Check summary table references and duplicates

import type { Bundle } from "../types/bundle.ts";

export function run(bundle: Bundle): void {
  const summaryTables = bundle.summary_tables;
  
  if (!summaryTables) {
    return; // No summary tables to validate
  }
  
  // Build set of valid table names from this bundle
  const validTables = new Set<string>();
  for (const table of bundle.tables) {
    validTables.add(table.name);
  }
  
  // Check for duplicate dashboard_var in summary tables
  const dashboardVars = new Set<string>();
  for (const summaryTable of summaryTables) {
    if (dashboardVars.has(summaryTable.dashboard_var)) {
      throw new Error(
        `Duplicated-Summary-Dashboard-Var bundle=${bundle.name} ` +
        `summary_table=${summaryTable.name} ` +
        `dashboard_var=${summaryTable.dashboard_var} ` +
        `url=${bundle.base_url}`
      );
    }
    dashboardVars.add(summaryTable.dashboard_var);
  }
  
  // Check each summary table references a valid parent
  for (const summaryTable of summaryTables) {
    if (!validTables.has(summaryTable.parent_table_name)) {
      throw new Error(
        `Invalid-Parent-Table-Reference bundle=${bundle.name} ` +
        `summary_table=${summaryTable.name} ` +
        `parent_table_name=${summaryTable.parent_table_name} ` +
        `url=${bundle.base_url}`
      );
    }
  }
}