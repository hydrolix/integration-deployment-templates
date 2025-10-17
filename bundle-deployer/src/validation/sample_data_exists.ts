// Validation: Check that transforms have sample data

import type { Bundle } from "../types/bundle.ts";
import { getErrorMessage } from "../utils/error.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      const fullPath = `${base}/${transform.path}`;
      
      const content = await Deno.readTextFile(fullPath);
      
      let transformJson;
      try {
        transformJson = JSON.parse(content);
      } catch (e) {
        throw new Error(
          `Failed to parse transform JSON full_path=${fullPath}: error=${getErrorMessage(e)}`
        );
      }
      
      const sampleData = transformJson?.settings?.sample_data;
      
      // Check if it's a non-empty object
      if (sampleData && typeof sampleData === 'object' && Object.keys(sampleData).length > 0) {
        continue;
      }
      
      // Check if it's a non-empty string
      if (typeof sampleData === 'string' && sampleData.length > 0) {
        continue;
      }
      
      throw new Error(
        `No Sample data in transformation full_path=${fullPath}`
      );
    }
  }
}