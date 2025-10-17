// Template for validation modules
// Copy this file and implement your validation logic

import type { Bundle } from "../types/bundle.ts";

// For validations that only need the bundle object
export function run(bundle: Bundle): void {
  // Example validation logic:
  
  // Check something
  if (somethingIsWrong) {
    throw new Error("Descriptive error message explaining what's wrong");
  }
  
  // Check multiple things
  for (const table of bundle.tables) {
    if (table.name === "") {
      throw new Error(`Table name cannot be empty`);
    }
  }
  
  // All good - no need to return anything
}

// Alternative: For validations that need to read files
export async function runAsync(base: string, bundle: Bundle): Promise<void> {
  // Read a file
  const filePath = `${base}/somefile.json`;
  
  try {
    const content = await Deno.readTextFile(filePath);
    const data = JSON.parse(content);
    
    // Validate the content
    if (!data.someProperty) {
      throw new Error("Missing required property");
    }
  } catch (e) {
    throw new Error(`Failed to validate ${filePath}: ${e.message}`);
  }
}

// Example: Checking checksums
export async function runWithChecksums(base: string, bundle: Bundle): Promise<void> {
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      if (!transform.sha256) {
        continue; // Skip if no checksum specified
      }
      
      const filePath = `${base}/${transform.path}`;
      const fileContent = await Deno.readFile(filePath);
      
      // Calculate SHA256
      const hashBuffer = await crypto.subtle.digest("SHA-256", fileContent);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
      
      if (hashHex !== transform.sha256) {
        throw new Error(
          `Checksum mismatch for ${transform.path}: expected ${transform.sha256}, got ${hashHex}`
        );
      }
    }
  }
}

// Example: Walking directories
export async function runWithFiles(base: string, bundle: Bundle): Promise<void> {
  import { walk } from "@std/fs";
  
  for await (const entry of walk(base, { maxDepth: 2 })) {
    if (entry.isFile && entry.name.endsWith(".json")) {
      // Do something with the file
      console.log(`Found: ${entry.path}`);
    }
  }
}