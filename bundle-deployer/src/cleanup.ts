// Cleanup script for Hydrolix resources
// Usage:
//   deno run --allow-all src/cleanup.ts --functions
//   deno run --allow-all src/cleanup.ts --dictionaries
//   deno run --allow-all src/cleanup.ts --tables
//   deno run --allow-all src/cleanup.ts --all

import { getErrorMessage } from "./utils/error.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const BUNDLE_TESTING_USERNAME = Deno.env.get("BUNDLE_TESTING_USERNAME") || "";
const BUNDLE_TESTING_PASSWORD = Deno.env.get("BUNDLE_TESTING_PASSWORD") || "";

const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";
const PROJ_NAME = "sample_project";

const HTTP_TIMEOUT = 120000;

const args = Deno.args;
const DELETE_FUNCTIONS = args.includes("--functions") || args.includes("--all");
const DELETE_DICTIONARIES = args.includes("--dictionaries") || args.includes("--all");
const DELETE_TABLES = args.includes("--tables") || args.includes("--all");
const DRY_RUN = args.includes("--dry-run");

async function getAuthToken(): Promise<string> {
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/login`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: BUNDLE_TESTING_USERNAME,
        password: BUNDLE_TESTING_PASSWORD,
      }),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      throw new Error(`Auth failed: ${response.statusText}`);
    }
    
    const json = await response.json();
    const token = json?.auth_token?.access_token;
    
    if (!token) {
      throw new Error("Could not find token in payload");
    }
    
    return token;
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to get auth token: ${getErrorMessage(e)}`);
  }
}

async function deleteFunctions(bearerToken: string): Promise<void> {
  console.log("\n🗑️  Deleting all functions...");
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (!listResponse.ok) {
      throw new Error(`Failed to list functions: ${listResponse.statusText}`);
    }
    
    const responseData = await listResponse.json();
    
    let functions: Array<{ name: string; uuid: string }> = [];
    if (Array.isArray(responseData)) {
      functions = responseData;
    } else if (responseData?.functions && Array.isArray(responseData.functions)) {
      functions = responseData.functions;
    } else if (responseData?.data && Array.isArray(responseData.data)) {
      functions = responseData.data;
    } else {
      console.log(`  Response format: ${JSON.stringify(responseData, null, 2)}`);
      throw new Error("Unexpected response format - see above");
    }
    
    if (functions.length === 0) {
      console.log("  No functions to delete");
      return;
    }
    
    console.log(`  Found ${functions.length} function(s)`);
    
    for (const fn of functions) {
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${fn.name}`);
        continue;
      }
      
      const deleteUrl = `${listUrl}${fn.uuid}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${fn.name}`);
        } else {
          console.warn(`  ⚠️  Failed to delete ${fn.name}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${fn.name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${functions.length} function(s)`);
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
    
    let dictionaries: Array<{ name: string; uuid: string }> = [];
    if (Array.isArray(responseData)) {
      dictionaries = responseData;
    } else if (responseData?.dictionaries && Array.isArray(responseData.dictionaries)) {
      dictionaries = responseData.dictionaries;
    } else if (responseData?.data && Array.isArray(responseData.data)) {
      dictionaries = responseData.data;
    } else {
      console.log(`  Response format: ${JSON.stringify(responseData, null, 2)}`);
      throw new Error("Unexpected response format - see above");
    }
    
    if (dictionaries.length === 0) {
      console.log("  No dictionaries to delete");
      return;
    }
    
    console.log(`  Found ${dictionaries.length} dictionar(y/ies)`);
    
    for (const dict of dictionaries) {
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${dict.name}`);
        continue;
      }
      
      const deleteUrl = `${listUrl}${dict.uuid}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${dict.name}`);
        } else {
          console.warn(`  ⚠️  Failed to delete ${dict.name}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${dict.name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${dictionaries.length} dictionar(y/ies)`);
  } catch (e) {
    throw new Error(`Failed to delete dictionaries: ${getErrorMessage(e)}`);
  }
}

async function deleteTables(bearerToken: string): Promise<void> {
  console.log("\n🗑️  Deleting all tables...");
  console.log("  ⚠️  WARNING: This will delete ALL data in tables!");
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (!listResponse.ok) {
      throw new Error(`Failed to list tables: ${listResponse.statusText}`);
    }
    
    const responseData = await listResponse.json();
    
    // Handle different response formats
    let tables: Array<{ name: string; uuid: string }> = [];
    if (Array.isArray(responseData)) {
      tables = responseData;
    } else if (responseData?.tables && Array.isArray(responseData.tables)) {
      tables = responseData.tables;
    } else if (responseData?.data && Array.isArray(responseData.data)) {
      tables = responseData.data;
    } else {
      console.log(`  Response format: ${JSON.stringify(responseData, null, 2)}`);
      throw new Error("Unexpected response format - see above");
    }
    
    if (tables.length === 0) {
      console.log("  No tables to delete");
      return;
    }
    
    console.log(`  Found ${tables.length} table(s)`);
    
    for (const table of tables) {
      if (DRY_RUN) {
        console.log(`  [DRY RUN] Would delete: ${table.name}`);
        continue;
      }
      
      const deleteUrl = `${listUrl}/${table.uuid}`;
      
      try {
        const deleteResponse = await fetch(deleteUrl, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${bearerToken}` },
        });
        
        if (deleteResponse.ok) {
          console.log(`  ✓ Deleted: ${table.name}`);
        } else {
          console.warn(`  ⚠️  Failed to delete ${table.name}: ${deleteResponse.statusText}`);
        }
      } catch (e) {
        console.warn(`  ⚠️  Error deleting ${table.name}: ${getErrorMessage(e)}`);
      }
    }
    
    console.log(`✓ Deleted ${tables.length} table(s)`);
  } catch (e) {
    throw new Error(`Failed to delete tables: ${getErrorMessage(e)}`);
  }
}

async function main() {
  if (!DELETE_FUNCTIONS && !DELETE_DICTIONARIES && !DELETE_TABLES) {
    console.log("Usage:");
    console.log("  deno run --allow-all src/cleanup.ts --functions      # Delete all functions");
    console.log("  deno run --allow-all src/cleanup.ts --dictionaries   # Delete all dictionaries");
    console.log("  deno run --allow-all src/cleanup.ts --tables         # Delete all tables");
    console.log("  deno run --allow-all src/cleanup.ts --all            # Delete everything");
    console.log("  deno run --allow-all src/cleanup.ts --all --dry-run  # Show what would be deleted");
    Deno.exit(1);
  }
  
  console.log(`\n🧹 Cleanup Script for project: ${PROJ_NAME}`);
  console.log(`   Cluster: ${BUNDLE_TESTING_CLUSTER}`);
  
  if (DRY_RUN) {
    console.log("   🔍 DRY RUN MODE - Nothing will actually be deleted\n");
  }
  
  try {
    const bearerToken = await getAuthToken();
    console.log("✓ Authenticated successfully");
    
    if (DELETE_FUNCTIONS) {
      await deleteFunctions(bearerToken);
    }
    
    if (DELETE_DICTIONARIES) {
      await deleteDictionaries(bearerToken);
    }
    
    if (DELETE_TABLES) {
      await deleteTables(bearerToken);
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