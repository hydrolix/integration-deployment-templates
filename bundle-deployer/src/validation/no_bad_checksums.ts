// Validation: Verify SHA256 checksums of files

import type { Bundle } from "../types/bundle.ts";

export async function run(base: string, bundle: Bundle): Promise<void> {
  // Check dashboard checksum
  const dashboardPath = `${base}/${bundle.dashboard.path}`;
  await checkChecksum(dashboardPath, bundle.dashboard.sha256);
  
  // Check transform checksums
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      const fullPath = `${base}/${transform.path}`;
      await checkChecksum(fullPath, transform.sha256);
    }
  }
  
  // Check summary table checksums
  if (bundle.summary_tables) {
    for (const summaryTable of bundle.summary_tables) {
      const fullPath = `${base}/${summaryTable.sql.path}`;
      await checkChecksum(fullPath, summaryTable.sql.sha256);
    }
  }
}

async function checkChecksum(filePath: string, checksum?: string): Promise<void> {
  const content = await Deno.readTextFile(filePath);
  
  // If checksum is provided, verify it
  if (checksum) {
    const calculatedHash = await generateSha256(content);
    
    if (calculatedHash !== checksum) {
      throw new Error(
        `SHA256 ${calculatedHash} does not match for local file ${filePath}`
      );
    }
  }
}

async function generateSha256(input: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}