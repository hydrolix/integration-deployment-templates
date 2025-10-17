// Validation: Check that transform JSON files are valid

import type { Bundle } from "../types/bundle.ts";
import { getErrorMessage } from "../utils/error.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  for (const table of bundle.tables) {
    const transformNames = new Set<string>();
    
    for (const transform of table.transforms) {
      const fullPath = `${base}/${transform.path}`;
      
      // Read the transform file
      const content = await Deno.readTextFile(fullPath);
      
      // Parse as JSON
      let transformJson;
      try {
        transformJson = JSON.parse(content);
      } catch (e) {
        throw new Error(
          `Transform file is not valid JSON: path=${fullPath} error=${getErrorMessage(e)}`
        );
      }
      
      // Check for non-empty name field
      const name = transformJson.name;
      
      if (name === undefined || name === null) {
        throw new Error(
          `Transform file missing required 'name' field: path=${fullPath}`
        );
      }
      
      if (typeof name !== 'string') {
        throw new Error(
          `Transform file 'name' field is not a string: path=${fullPath}`
        );
      }
      
      if (name.trim().length === 0) {
        throw new Error(
          `Transform file has empty 'name' field: path=${fullPath}`
        );
      }
      
      // Check for duplicate transform names
      if (transformNames.has(name)) {
        throw new Error(
          `Duplicated transform name '${name}' path=${fullPath}`
        );
      }
      transformNames.add(name);
      
      // Check subtype if present
      if (transformJson.subtype !== undefined) {
        const subtype = transformJson.subtype;
        
        if (typeof subtype !== 'string') {
          throw new Error(
            `Transform file 'subtype' field is not a string: path=${fullPath}`
          );
        }
        
        if (subtype !== 'firehose') {
          throw new Error(
            `Transform file has invalid subtype '${subtype}', must be 'firehose': path=${fullPath}`
          );
        }
      }
    }
  }
}