// Main entry point for bundle validator with alert rules support

import { walk } from "@std/fs";
import type { Bundle, Output } from "./types/bundle.ts";
import { validateBundle } from "./types/bundle.ts";
import { getErrorMessage } from "./utils/error.ts";
import * as grafana from "./grafana/container.ts";
import * as headless from "./headless_browser.ts";
import * as deploy from "./deploy.ts";
import * as deployOnlyDashboard from "./deploy_only_dashboard.ts";

// Import validation modules
import * as naming_is_valid from "./validation/naming_is_valid.ts";
import * as no_duplicate_tokens from "./validation/no_duplicate_tokens.ts";
import * as valid_base_url from "./validation/valid_base_url.ts";
import * as dashboard_is_valid from "./validation/dashboard_is_valid.ts";
import * as alert_rules_are_valid from "./validation/alert_rules_are_valid.ts";
import * as no_bad_checksums from "./validation/no_bad_checksums.ts";
import * as sample_data_exists from "./validation/sample_data_exists.ts";
import * as transforms_are_valid from "./validation/transforms_are_valid.ts";
import * as summary_table from "./validation/summary_table.ts";
import * as no_global_duplicates from "./validation/no_global_duplicates.ts";
import * as check_dependencies from "./validation/check_dependencies.ts";

// Environment variables and CLI args
const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const BUNDLE_TESTING_USERNAME = Deno.env.get("BUNDLE_TESTING_USERNAME") || "";
const BUNDLE_TESTING_PASSWORD = Deno.env.get("BUNDLE_TESTING_PASSWORD") || "";

const args = Deno.args;
const SCAN_WIP = args.includes("--wip");
const IS_LOCAL = args.includes("--local");
const IS_LOCAL_DASHBOARD_ONLY = args.includes("--local-dashboard-only");
const FOR_MARKETPLACE = args.includes("--marketplace");
const DUMP_OUTPUT = args.includes("--output");
const PRODUCTION_MODE = args.includes("--production");
const MATCH_ONLY = args.find(arg => !arg.startsWith("--")) || "";

async function main() {
  let bundlesChecked = 0;
  
  const bundleList = await findBundleFiles();
  
  const finalBundleList: Bundle[] = [];
  const allBundleList: Bundle[] = [];
  
  for (const bundlePath of bundleList) {
    const bundle = await fileToBundle(bundlePath);
    
    // Track all bundles for global duplicate check
    allBundleList.push(bundle);
    
    // Filter by name if specified
    if (MATCH_ONLY && !bundle.name.includes(MATCH_ONLY)) {
      console.log(`Ignoring ${bundle.name} ${MATCH_ONLY}`);
      continue;
    }
    
    const baseDir = bundlePath.replace("./", "").replace("/bundle.json", "");
    console.log(`Testing ${bundle.name}`);
    
    bundlesChecked++;
    
    try {
      await validateBundleFull(baseDir, bundle);
    } catch (e) {
      console.error(`ERROR: Failed bundle validation: ${getErrorMessage(e)}`);
      Deno.exit(1);
    }
    
    console.log(`Bundle=${JSON.stringify(bundle, null, 2)}`);
    finalBundleList.push(bundle);
  }
  
  if (bundlesChecked === 0) {
    console.error("ERROR: No bundles were checked - nothing matched the filter.");
    Deno.exit(1);
  }
  
  console.log("Final check on all of the bundles for duplicated tokens...");
  no_global_duplicates.run(allBundleList);
  
  console.log("SUCCESS");
  Deno.exit(0);
}

async function validateBundleFull(base: string, bundle: Bundle): Promise<void> {
  console.log(`Base=${base} bundle=${JSON.stringify(bundle, null, 2)}`);
  
  const output: Output = {
    cluster_domain: "",
    project_name: "",
    grafana_domain: "",
    datalink: "",
    dashboard_id: "",
    tables: [],
  };
  
  // Run all validation checks
  valid_base_url.run(base, bundle);
  no_duplicate_tokens.run(bundle);
  naming_is_valid.run(bundle);
  await no_bad_checksums.run(base, bundle);
  await transforms_are_valid.run(base, bundle);
  await dashboard_is_valid.run(base, bundle);
  await alert_rules_are_valid.run(base, bundle);
  await sample_data_exists.run(base, bundle);
  summary_table.run(bundle);
  
  // Check dependencies (warnings only, doesn't fail validation)
  await check_dependencies.run(base, bundle);
  
  if (IS_LOCAL_DASHBOARD_ONLY) {
    // Kill previous container if it exists
    console.log("!!!!! LOCAL DASHBOARD ONLY WORKING");
    await grafana.kill().catch(() => {});
    
    await grafana.start();
    
    const dashboardId = await deployOnlyDashboard.run(base, bundle, output);
    console.log(`Dashboard_id=${dashboardId}`);
    
    console.log("Checking the Grafana dashboard with headless Chrome");
    const [datasourceErrorCount, nodataErrorCount] = await headless.run(dashboardId);
    
    console.log(`Dashboard Errors=${datasourceErrorCount} NoDataErrors=${nodataErrorCount}`);
    
    if (datasourceErrorCount > 0 || nodataErrorCount > 0) {
      throw new Error(
        `Dashboard Errors=${datasourceErrorCount} NoDataErrors=${nodataErrorCount}`
      );
    }
  } else if (IS_LOCAL) {
    // Kill previous container if it exists
    await grafana.kill().catch(() => {});
    
    await grafana.start();
    
    const dashboardId = await deploy.run(base, bundle, output);
    console.log(`Dashboard_id=${dashboardId}`);
    
    console.log("Checking the Grafana dashboard with headless Chrome");
    const [datasourceErrorCount, nodataErrorCount] = await headless.run(dashboardId);
    
    console.log(`Dashboard Errors=${datasourceErrorCount} NoDataErrors=${nodataErrorCount}`);
    
    if (datasourceErrorCount > 0 || nodataErrorCount > 0) {
      throw new Error(
        `Dashboard Errors=${datasourceErrorCount} NoDataErrors=${nodataErrorCount}`
      );
    }
  }
  
  if (DUMP_OUTPUT) {
    console.log("OUTPUT FOR TRAFFIC GENERATION:\n\n" + JSON.stringify(output, null, 2));
  }
  
  console.log("SUCCESS");
}

async function findBundleFiles(): Promise<string[]> {
  const searchPath = SCAN_WIP ? "./WIP" : ".";
  const bundles: string[] = [];
  
  for await (const entry of walk(searchPath, { maxDepth: 2 })) {
    if (entry.isFile && entry.name === "bundle.json") {
      bundles.push(entry.path);
    }
  }
  
  return bundles;
}

async function fileToBundle(filePath: string): Promise<Bundle> {
  try {
    const content = await Deno.readTextFile(filePath);
    const bundle = JSON.parse(content) as Bundle;
    
    // Validate structure
    validateBundle(bundle);
    
    return bundle;
  } catch (e) {
    throw new Error(`Failed to read/parse bundle file ${filePath}: ${getErrorMessage(e)}`);
  }
}

// Run the main function
if (import.meta.main) {
  main();
}