// Check if required functions and dictionaries exist in Hydrolix

import type { Bundle } from "./types/bundle.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";
const PROJ_NAME = "sample_project";

export async function checkDependenciesExist(
  bearerToken: string,
  bundle: Bundle
): Promise<void> {
  const missingFunctions: string[] = [];
  const missingDictionaries: string[] = [];
  
  // Check functions
  if (bundle.dependencies?.hydrolix?.required_functions) {
    const functionsUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
    
    try {
      const response = await fetch(functionsUrl, {
        headers: { 'Authorization': `Bearer ${bearerToken}` },
      });
      
      if (response.ok) {
        const existingFunctions = await response.json() as Array<{ name: string }>;
        const existingNames = new Set(existingFunctions.map(f => f.name));
        
        for (const fn of bundle.dependencies.hydrolix.required_functions) {
          const fullName = `${PROJ_NAME}_${fn.name}`;
          if (!existingNames.has(fullName)) {
            missingFunctions.push(fn.name);
          }
        }
      } else {
        throw new Error(`Failed to list functions: ${response.statusText}`);
      }
    } catch (e) {
      throw new Error(`Failed to check functions: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  
  // Check dictionaries
  if (bundle.dependencies?.hydrolix?.required_dictionaries) {
    const dictsUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
    
    try {
      const response = await fetch(dictsUrl, {
        headers: { 'Authorization': `Bearer ${bearerToken}` },
      });
      
      if (response.ok) {
        const existingDicts = await response.json() as Array<{ name: string }>;
        const existingNames = new Set(existingDicts.map(d => d.name));
        
        for (const dict of bundle.dependencies.hydrolix.required_dictionaries) {
          const fullName = `${PROJ_NAME}_${dict.name}`;
          if (!existingNames.has(fullName)) {
            missingDictionaries.push(dict.name);
          }
        }
      } else {
        throw new Error(`Failed to list dictionaries: ${response.statusText}`);
      }
    } catch (e) {
      throw new Error(`Failed to check dictionaries: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  
  // Throw error if anything is missing
  if (missingFunctions.length > 0 || missingDictionaries.length > 0) {
    const errorParts: string[] = [];
    
    if (missingFunctions.length > 0) {
      errorParts.push(`Missing functions: ${missingFunctions.join(', ')}`);
      errorParts.push(`  Expected as: ${missingFunctions.map(f => `${PROJ_NAME}_${f}`).join(', ')}`);
    }
    
    if (missingDictionaries.length > 0) {
      errorParts.push(`Missing dictionaries: ${missingDictionaries.join(', ')}`);
      errorParts.push(`  Expected as: ${missingDictionaries.map(d => `${PROJ_NAME}_${d}`).join(', ')}`);
    }
    
    errorParts.push('');
    errorParts.push('In production mode, these must be created before bundle deployment.');
    errorParts.push('Either:');
    errorParts.push('  1. Create them manually in Hydrolix');
    errorParts.push('  2. Run without --production flag to auto-create them');
    
    throw new Error(errorParts.join('\n'));
  }
  
  console.log('✓ All required dependencies exist in production environment');
}