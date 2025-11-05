// Cleanup script for Hydrolix resources
// Usage:
//   deno run --allow-all src/cleanup.ts --functions mcdn_test
//   deno run --allow-all src/cleanup.ts --dictionaries mcdn_test
//   deno run --allow-all src/cleanup.ts --tables mcdn_test
//   deno run --allow-all src/cleanup.ts --all mcdn_test
//   deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run

import { getErrorMessage } from "./utils/error.ts";
import * as hdx from "./hdx.ts";
import type { Bundle } from "./types/bundle.ts";

const args = Deno.args;
const DELETE_FUNCTIONS = args.includes("--functions") || args.includes("--all");
const DELETE_DICTIONARIES = args.includes("--dictionaries") || args.includes("--all");
const DELETE_DICTIONARY_FILES = args.includes("--dictionary-files") || args.includes("--all");  // Include in --all
const DELETE_TABLES = args.includes("--tables") || args.includes("--all");
const DRY_RUN = args.includes("--dry-run");

// Get bundle name (first non-flag argument)
const BUNDLE_NAME = args.find(arg => !arg.startsWith("--")) || "";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";
const PROJ_NAME = "sample_project";

async function deleteFunctions(bearerToken: string, bundle: Bundle | null): Promise<void> {
  console.log("\n🗑️  Deleting functions...");
  
  // Build list of functions to delete
  let functionsToDelete: Set<string> | null = null;
  if (bundle) {
    functionsToDelete = new Set<string>();
    const declared = bundle.dependencies?.hydrolix?.required_functions || [];
    for (const fnName of declared) {
      functionsToDelete.add(fnName);  // Just the base name, no prefix!
    }
    
    // Also try to discover from bundle directory
    try {
      const discovered = await hdx.discoverFunctions(`my-bundles/${BUNDLE_NAME}`);
      for (const fnName of discovered) {
        functionsToDelete.add(fnName);  // Just the base name!
      }
    } catch {
      // Ignore discovery errors
    }
    
    if (functionsToDelete.size === 0) {
      console.log("  No functions to delete (none declared in bundle)");
      return;
    }
    
    console.log(`  Scope: ${functionsToDelete.size} function(s) from bundle`);
  } else {
    console.log("  Scope: ALL functions in project");
  }
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (!listResponse.ok) {
      throw new Error(`Failed to list functions: ${listResponse.statusText}`);
    }
    
    const responseData = await listResponse.json();
    
    // Handle paginated response
    let functions: Array<any> = [];
    if (responseData?.results && Array.isArray(responseData.results)) {
      functions = responseData.results;
    } else if (Array.isArray(responseData)) {
      functions = responseData;
    }
    
    if (functions.length === 0) {
      console.log("  No functions found on cluster");
      return;
    }
    
    console.log(`  Found ${functions.length} total function(s) on cluster`);
    
    let deleted = 0;
    for (const fn of functions) {
      const name = fn.name || fn.function_name || fn.id || 'unknown';
      const uuid = fn.uuid || fn.id || fn.uid || fn.function_id;
      
      // Filter by bundle if specified
      if (functionsToDelete && !functionsToDelete.has(name)) {
        continue;  // Skip - not in this bundle
      }
      
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${name}`);
        continue;
      }
      
      if (!uuid) {
        console.warn(`  ⚠️  Skipping ${name} - no UUID found`);
        continue;
      }
      
      const deleteUrl = `${listUrl}${uuid}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${name}`);
          deleted++;
        } else {
          console.warn(`  ⚠️  Failed to delete ${name}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${deleted} function(s)`);
  } catch (e) {
    throw new Error(`Failed to delete functions: ${getErrorMessage(e)}`);
  }
}

async function deleteDictionaries(bearerToken: string): Promise<void> {
  console.log("\n🗑️  Deleting all dictionary definitions...");
  console.log("  (Note: Uploaded dictionary files will NOT be deleted)");
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (!listResponse.ok) {
      throw new Error(`Failed to list dictionaries: ${listResponse.statusText}`);
    }
    
    const responseData = await listResponse.json();
    
    // Handle paginated response
    let dictionaries: Array<any> = [];
    if (responseData?.results && Array.isArray(responseData.results)) {
      dictionaries = responseData.results;
    } else if (Array.isArray(responseData)) {
      dictionaries = responseData;
    }
    
    if (dictionaries.length === 0) {
      console.log("  No dictionaries to delete");
      return;
    }
    
    console.log(`  Found ${dictionaries.length} dictionar(y/ies)`);
    
    let deleted = 0;
    for (const dict of dictionaries) {
      const name = dict.name || dict.dictionary_name || dict.id || 'unknown';
      const uuid = dict.uuid || dict.id || dict.uid || dict.dictionary_id;
      
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${name} (uuid: ${uuid})`);
        continue;
      }
      
      if (!uuid) {
        console.warn(`  ⚠️  Skipping ${name} - no UUID found`);
        continue;
      }
      
      const deleteUrl = `${listUrl}${uuid}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${name}`);
          deleted++;
        } else {
          console.warn(`  ⚠️  Failed to delete ${name}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${deleted} dictionar(y/ies)`);
  } catch (e) {
    throw new Error(`Failed to delete dictionaries: ${getErrorMessage(e)}`);
  }
}

async function deleteDictionaryFiles(bearerToken: string, bundle: Bundle | null): Promise<void> {
  console.log("\n🗑️  Deleting uploaded dictionary files...");
  console.log("  ⚠️  WARNING: This will delete uploaded CSV/YAML files!");
  
  // Build list of file names to delete
  let fileNamesToDelete: Set<string> | null = null;
  if (bundle) {
    fileNamesToDelete = new Set<string>();
    const declared = bundle.dependencies?.hydrolix?.required_dictionaries || [];
    
    // Discover dictionaries to get their actual file names
    const allDictNames: string[] = [...declared];
    try {
      const discovered = await hdx.discoverDictionaries(`my-bundles/${BUNDLE_NAME}`);
      allDictNames.push(...discovered);
    } catch {
      // Ignore discovery errors
    }
    
    // For each dictionary, add possible file names (with and without extensions)
    for (const dictName of allDictNames) {
      fileNamesToDelete.add(dictName);  // Base name
      fileNamesToDelete.add(`${dictName}.csv`);
      fileNamesToDelete.add(`${dictName}.yaml`);
      fileNamesToDelete.add(`${dictName}.yml`);
      fileNamesToDelete.add(`${dictName}.tsv`);
    }
    
    console.log(`  Scope: Files for ${allDictNames.length} dictionar(y/ies) from bundle`);
  } else {
    console.log("  Scope: ALL dictionary files in project");
  }
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/files/`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (!listResponse.ok) {
      throw new Error(`Failed to list dictionary files: ${listResponse.statusText}`);
    }
    
    const responseData = await listResponse.json();
    
    // Files might be just strings or objects
    let files: string[] = [];
    if (Array.isArray(responseData)) {
      files = responseData.map((f: any) => typeof f === 'string' ? f : f.name || f.id);
    } else if (responseData?.files) {
      files = Array.isArray(responseData.files) ? responseData.files : [];
    } else if (responseData?.results) {
      files = responseData.results.map((f: any) => typeof f === 'string' ? f : f.name || f.id);
    }
    
    if (files.length === 0) {
      console.log("  No dictionary files found on cluster");
      return;
    }
    
    console.log(`  Found ${files.length} total file(s) on cluster`);
    
    let deleted = 0;
    for (const fileName of files) {
      // Filter by bundle if specified
      if (fileNamesToDelete && !fileNamesToDelete.has(fileName)) {
        continue;  // Skip - not used by this bundle
      }
      
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${fileName}`);
        continue;
      }
      
      const deleteUrl = `${listUrl}${fileName}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${fileName}`);
          deleted++;
        } else {
          console.warn(`  ⚠️  Failed to delete ${fileName}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${fileName}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${deleted} file(s)`);
  } catch (e) {
    throw new Error(`Failed to delete dictionary files: ${getErrorMessage(e)}`);
  }
}

async function deleteTables(bearerToken: string, bundle: Bundle | null): Promise<void> {
  console.log("\n🗑️  Deleting tables...");
  console.log("  ⚠️  WARNING: This will delete table data!");
  
  // Build list of tables to delete
  let tablesToDelete: Set<string> | null = null;
  if (bundle) {
    tablesToDelete = new Set<string>();
    
    // Add main tables
    for (const table of bundle.tables) {
      tablesToDelete.add(table.name);
    }
    
    // Add summary tables
    if (bundle.summary_tables) {
      for (const summary of bundle.summary_tables) {
        tablesToDelete.add(summary.name);
      }
    }
    
    if (tablesToDelete.size === 0) {
      console.log("  No tables to delete (none declared in bundle)");
      return;
    }
    
    console.log(`  Scope: ${tablesToDelete.size} table(s) from bundle`);
  } else {
    console.log("  Scope: ALL tables in project");
  }
  
  try {
    const tableListText = await hdx.getTableList(bearerToken);
    const responseData = JSON.parse(tableListText);
    
    // Handle paginated response
    let tables: Array<{ name: string; uuid: string }> = [];
    if (responseData?.results && Array.isArray(responseData.results)) {
      tables = responseData.results;
    } else if (Array.isArray(responseData)) {
      tables = responseData;
    }
    
    if (tables.length === 0) {
      console.log("  No tables found on cluster");
      return;
    }
    
    console.log(`  Found ${tables.length} total table(s) on cluster`);
    
    let deleted = 0;
    for (const table of tables) {
      // Filter by bundle if specified
      if (tablesToDelete && !tablesToDelete.has(table.name)) {
        continue;  // Skip - not in this bundle
      }
      
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${table.name}`);
        continue;
      }
      
      try {
        await hdx.deleteTable(bearerToken, table.uuid);
        console.log(`  ✓ Deleted: ${table.name}`);
        deleted++;
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${table.name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${deleted} table(s)`);
  } catch (e) {
    throw new Error(`Failed to delete tables: ${getErrorMessage(e)}`);
  }
}

async function main() {
  if (!DELETE_FUNCTIONS && !DELETE_DICTIONARIES && !DELETE_TABLES && !DELETE_DICTIONARY_FILES) {
    console.log("Cleanup Script for Hydrolix Resources");
    console.log("\nUsage:");
    console.log("  deno run --allow-all src/cleanup.ts --functions mcdn_test       # Delete bundle's functions");
    console.log("  deno run --allow-all src/cleanup.ts --dictionaries mcdn_test    # Delete bundle's dictionaries");
    console.log("  deno run --allow-all src/cleanup.ts --dictionary-files mcdn_test # Delete bundle's dictionary files");
    console.log("  deno run --allow-all src/cleanup.ts --tables mcdn_test          # Delete bundle's tables");
    console.log("  deno run --allow-all src/cleanup.ts --all mcdn_test             # Delete everything for bundle");
    console.log("  deno run --allow-all src/cleanup.ts --all mcdn_test --dry-run   # Show what would be deleted");
    console.log("\nNote: --all includes functions, dictionaries, dictionary files, and tables");
    console.log("      Provide bundle name to delete only that bundle's resources (recommended)");
    console.log("      Omit bundle name to delete ALL resources in project (dangerous!)");
    Deno.exit(1);
  }
  
  console.log(`\n🧹 Cleanup Script for project: ${PROJ_NAME}`);
  console.log(`   Cluster: ${BUNDLE_TESTING_CLUSTER}`);
  
  // Load bundle if specified
  let bundle: Bundle | null = null;
  if (BUNDLE_NAME) {
    try {
      const bundlePath = `my-bundles/${BUNDLE_NAME}/bundle.json`;
      const content = await Deno.readTextFile(bundlePath);
      bundle = JSON.parse(content);
      console.log(`   Bundle: ${BUNDLE_NAME}`);
      console.log(`   Scope: Only resources from this bundle`);
    } catch (e) {
      console.error(`\n❌ Could not read bundle: ${getErrorMessage(e)}`);
      Deno.exit(1);
    }
  } else {
    console.log(`   ⚠️  WARNING: No bundle specified - will delete ALL resources!`);
    console.log(`   Press Ctrl+C to cancel...`);
    await new Promise(resolve => setTimeout(resolve, 5000));
  }
  
  if (DRY_RUN) {
    console.log("   🔍 DRY RUN MODE - Nothing will actually be deleted\n");
  }
  
  try {
    const bearerToken = await hdx.getAuthToken();
    console.log("✓ Authenticated successfully");
    
    if (DELETE_FUNCTIONS) {
      await deleteFunctions(bearerToken, bundle);
    }
    
    if (DELETE_DICTIONARIES) {
      await deleteDictionaries(bearerToken, bundle);
    }
    
    if (DELETE_DICTIONARY_FILES) {
      await deleteDictionaryFiles(bearerToken, bundle);
    }
    
    if (DELETE_TABLES) {
      await deleteTables(bearerToken, bundle);
    }
    
    console.log("\n✅ Cleanup complete!");
    Deno.exit(0);
  } catch (e) {
    console.error(`\n❌ Cleanup failed: ${getErrorMessage(e)}`);
    Deno.exit(1);
  }
}

if (import.meta.main) {
  main();
}