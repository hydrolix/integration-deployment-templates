// Validation: Check for duplicate table names and dashboard_vars within a bundle

import type { Bundle } from "../types/bundle.ts";

const MIN_TABLE_NAME = 3;

export function run(bundle: Bundle): void {
  const tables = new Set<string>();
  
  // Check every table name
  for (const table of bundle.tables) {
    // Validate table name length
    if (!table.name || table.name.length < MIN_TABLE_NAME) {
      throw new Error(
        `Missing or truncated table name '${JSON.stringify(table)}'`
      );
    }
    
    // Validate table name starts with a letter
    const firstChar = table.name.charAt(0);
    if (!/[a-zA-Z]/.test(firstChar)) {
      throw new Error(
        `Invalid table name '${table.name}' - must start with a letter`
      );
    }
    
    // Validate table name contains only alphanumeric characters and underscores
    if (!/^[a-zA-Z0-9_]+$/.test(table.name)) {
      throw new Error(
        `Invalid table name '${table.name}' - only letters, digits, and underscores allowed`
      );
    }
    
    // Check for duplicate table names
    if (tables.has(table.name)) {
      throw new Error(`Duplicate table name ${table.name}`);
    }
    
    tables.add(table.name);
  }
  
  console.log(`tables=${JSON.stringify([...tables])}`);
  
  // Check for duplicate dashboard_var values
  const tokens = new Set<string>();
  for (const table of bundle.tables) {
    if (tokens.has(table.dashboard_var)) {
      throw new Error(`Duplicate database_var ${table.dashboard_var}`);
    }
    tokens.add(table.dashboard_var);
  }
}