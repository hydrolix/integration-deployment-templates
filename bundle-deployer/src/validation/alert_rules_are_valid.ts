// Validate alert rules file structure

import type { Bundle } from "../types/bundle.ts";
import { getErrorMessage } from "../utils/error.ts";

interface AlertRulesFile {
  apiVersion: number;
  groups: Array<{
    orgId?: number;
    name: string;
    folder: string;
    interval: string;
    rules: Array<{
      uid: string;
      title: string;
      condition: string;
      data: unknown[];
      [key: string]: unknown;
    }>;
  }>;
}

export async function run(base: string, bundle: Bundle): Promise<void> {
  if (!bundle.alert_rules) {
    console.log("ℹ️  No alert rules defined in bundle (optional)");
    return;
  }
  
  const alertRulesPath = `${base}/${bundle.alert_rules.path}`;
  
  console.log(`Validating alert rules at ${alertRulesPath}...`);
  
  try {
    const content = await Deno.readTextFile(alertRulesPath);
    const alertRules = JSON.parse(content) as AlertRulesFile;
    
    // Validate structure
    if (!alertRules.apiVersion) {
      throw new Error("alert_rules.apiVersion is required");
    }
    
    if (!Array.isArray(alertRules.groups)) {
      throw new Error("alert_rules.groups must be an array");
    }
    
    if (alertRules.groups.length === 0) {
      throw new Error("alert_rules.groups must contain at least one group");
    }
    
    // Validate each group
    for (let i = 0; i < alertRules.groups.length; i++) {
      const group = alertRules.groups[i];
      
      if (!group.name) {
        throw new Error(`Group ${i}: name is required`);
      }
      
      if (!group.folder) {
        throw new Error(`Group ${i} (${group.name}): folder is required`);
      }
      
      if (!group.interval) {
        throw new Error(`Group ${i} (${group.name}): interval is required`);
      }
      
      if (!Array.isArray(group.rules)) {
        throw new Error(`Group ${i} (${group.name}): rules must be an array`);
      }
      
      if (group.rules.length === 0) {
        throw new Error(`Group ${i} (${group.name}): rules must contain at least one rule`);
      }
      
      // Validate each rule
      for (let j = 0; j < group.rules.length; j++) {
        const rule = group.rules[j];
        
        if (!rule.uid) {
          throw new Error(`Group ${group.name}, Rule ${j}: uid is required`);
        }
        
        if (!rule.title) {
          throw new Error(`Group ${group.name}, Rule ${j}: title is required`);
        }
        
        if (!rule.condition) {
          throw new Error(`Group ${group.name}, Rule ${j}: condition is required`);
        }
        
        if (!Array.isArray(rule.data)) {
          throw new Error(`Group ${group.name}, Rule ${j}: data must be an array`);
        }
      }
    }
    
    console.log(`✓ Alert rules file is valid (${alertRules.groups.length} group(s), ${getTotalRules(alertRules)} rule(s))`);
    
  } catch (e) {
    throw new Error(`Failed to validate alert rules at ${alertRulesPath}: ${getErrorMessage(e)}`);
  }
}

function getTotalRules(alertRules: AlertRulesFile): number {
  return alertRules.groups.reduce((sum, group) => sum + group.rules.length, 0);
}