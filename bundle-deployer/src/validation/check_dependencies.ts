// Validation: Check that functions/dictionaries have local files and are used in SQL

import type { Bundle } from "../types/bundle.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  const declaredFunctions = new Set<string>();
  const declaredDictionaries = new Set<string>();
  
  // Collect declared dependencies
  if (bundle.dependencies?.hydrolix) {
    bundle.dependencies.hydrolix.required_functions?.forEach(fnName => {
      declaredFunctions.add(fnName);
    });
    
    bundle.dependencies.hydrolix.required_dictionaries?.forEach(dictName => {
      declaredDictionaries.add(dictName);
    });
  }
  
  // Check that local files exist for each declared dependency
  console.log("Checking local function files...");
  for (const fnName of declaredFunctions) {
    const jsonPath = `${base}/functions/${fnName}.json`;
    try {
      await Deno.stat(jsonPath);
      console.log(`  ✓ Function file exists: functions/${fnName}.json`);
    } catch {
      console.warn(`  ⚠️  WARNING: Function '${fnName}' declared but no file: functions/${fnName}.json`);
    }
  }
  
  console.log("Checking local dictionary files...");
  for (const dictName of declaredDictionaries) {
    const jsonPath = `${base}/dictionaries/${dictName}.json`;
    
    // Check for JSON definition
    let hasJson = false;
    try {
      await Deno.stat(jsonPath);
      hasJson = true;
      console.log(`  ✓ Dictionary definition exists: dictionaries/${dictName}.json`);
    } catch {
      console.warn(`  ⚠️  WARNING: Dictionary '${dictName}' declared but no definition: dictionaries/${dictName}.json`);
    }
    
    // Check for data file (CSV, YAML, or other)
    if (hasJson) {
      let foundDataFile = false;
      const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
      
      for (const ext of possibleExtensions) {
        const dataPath = `${base}/dictionaries/${dictName}.${ext}`;
        try {
          await Deno.stat(dataPath);
          console.log(`  ✓ Dictionary data file exists: dictionaries/${dictName}.${ext}`);
          foundDataFile = true;
          break;
        } catch {
          // Try next extension
        }
      }
      
      if (!foundDataFile) {
        console.warn(`  ⚠️  WARNING: Dictionary '${dictName}' has definition but no data file (checked .csv, .yaml, .yml, .tsv)`);
      }
    }
  }
  
  // Check each transform's SQL for function/dictionary usage
  console.log("Checking SQL references in transforms...");
  const usedFunctions = new Set<string>();
  const usedDictionaries = new Set<string>();
  
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      const fullPath = `${base}/${transform.path}`;
      const content = await Deno.readTextFile(fullPath);
      const transformJson = JSON.parse(content);
      
      const sql = transformJson?.settings?.sql_transform;
      if (!sql || typeof sql !== 'string') {
        continue;
      }
      
      // Check if declared functions are used
      for (const functionName of declaredFunctions) {
        // Match function calls: functionName(
        const regex = new RegExp(`\\b${functionName}\\s*\\(`, 'g');
        if (regex.test(sql)) {
          usedFunctions.add(functionName);
        }
      }
      
      // Check for dictGet/dictGetString calls
      const dictGetPattern = /dict(?:Get|GetString|GetOrDefault)\s*\(\s*['"]([^'"]+)['"]/gi;
      const dictMatches = [...sql.matchAll(dictGetPattern)];
      
      for (const match of dictMatches) {
        const dictName = match[1];
        usedDictionaries.add(dictName);
        
        if (!declaredDictionaries.has(dictName)) {
          console.warn(
            `  ⚠️  WARNING: Transform ${transform.path} uses dictionary '${dictName}' ` +
            `but it's not declared in dependencies.hydrolix.required_dictionaries`
          );
        }
      }
    }
  }
  
  // Report declared but unused dependencies
  for (const fnName of declaredFunctions) {
    if (!usedFunctions.has(fnName)) {
      console.warn(`  ⚠️  INFO: Function '${fnName}' is declared but not used in any transforms`);
    }
  }
  
  for (const dictName of declaredDictionaries) {
    if (!usedDictionaries.has(dictName)) {
      console.warn(`  ⚠️  INFO: Dictionary '${dictName}' is declared but not used in any transforms`);
    }
  }
  
  console.log("✓ Dependency validation complete");
}