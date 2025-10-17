// Validation: Check that SQL functions/dictionaries are declared in dependencies

import type { Bundle } from "../types/bundle.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  const declaredFunctions = new Set<string>();
  const declaredDictionaries = new Set<string>();
  
  // Collect declared dependencies
  if (bundle.dependencies?.hydrolix) {
    bundle.dependencies.hydrolix.required_functions?.forEach(fn => {
      declaredFunctions.add(fn.name);
    });
    
    bundle.dependencies.hydrolix.required_dictionaries?.forEach(dict => {
      declaredDictionaries.add(dict.name);
    });
  }
  
  // Check each transform's SQL
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      const fullPath = `${base}/${transform.path}`;
      const content = await Deno.readTextFile(fullPath);
      const transformJson = JSON.parse(content);
      
      const sql = transformJson?.settings?.sql_transform;
      if (!sql || typeof sql !== 'string') {
        continue;
      }
      
      // Check for function calls ending in _breadcrumbs, _extract, etc.
      const functionPattern = /(\w+_(?:breadcrumbs|extract|parse))\s*\(/gi;
      const functionMatches = [...sql.matchAll(functionPattern)];
      
      for (const match of functionMatches) {
        const functionName = match[1];
        if (!declaredFunctions.has(functionName)) {
          console.warn(
            `⚠ WARNING: Transform ${transform.path} uses function '${functionName}' ` +
            `but it's not declared in dependencies.hydrolix.required_functions`
          );
        }
      }
      
      // Check for dictGet calls
      const dictGetPattern = /dictGet\s*\(\s*['"]([^'"]+)['"]/gi;
      const dictMatches = [...sql.matchAll(dictGetPattern)];
      
      for (const match of dictMatches) {
        const dictName = match[1];
        if (!declaredDictionaries.has(dictName)) {
          console.warn(
            `⚠ WARNING: Transform ${transform.path} uses dictionary '${dictName}' ` +
            `but it's not declared in dependencies.hydrolix.required_dictionaries`
          );
        }
      }
    }
  }
}