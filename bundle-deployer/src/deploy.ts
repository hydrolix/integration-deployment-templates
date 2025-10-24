// Main deployment orchestration with alert rules support
// COMPLETE FILE - Replace your entire deploy.ts with this

import type { Bundle, Output, OutputTable, OutputTransformation } from "./types/bundle.ts";
import { getErrorMessage } from "./utils/error.ts";
import * as grafana from "./grafana/interface.ts";
import * as hdx from "./hdx.ts";
import * as hdxCheck from "./hdx_check_dependencies.ts";
import { GRAFANA_LOCATION } from "./grafana/container.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const PRODUCTION_MODE = Deno.args.includes("--production");
const TABLE_READY_DELAY_SECS = 30;
const DATA_READY_DELAY_SECS = 30;

export async function run(
  base: string,
  bundle: Bundle,
  output: Output
): Promise<string> {
  const bearerToken = await hdx.getAuthToken();
  const projectName = hdx.createProjectName();

  // Create functions and dictionaries BEFORE creating tables/transforms
  if (bundle.dependencies?.hydrolix) {
    // Process functions
    if (bundle.dependencies.hydrolix.required_functions) {
      console.log(`\nProcessing ${bundle.dependencies.hydrolix.required_functions.length} required function(s)...`);
      
      for (const functionName of bundle.dependencies.hydrolix.required_functions) {
        try {
          await hdx.checkAndCreateFunction(bearerToken, functionName, base);
        } catch (e) {
          console.warn(`⚠️  WARNING: Failed to create function ${functionName}: ${getErrorMessage(e)}`);
          console.warn(`⚠️  Continuing anyway - transforms may be invalid until function is added`);
        }
      }
    }

    // Process dictionaries
    if (bundle.dependencies.hydrolix.required_dictionaries) {
      console.log(`\nProcessing ${bundle.dependencies.hydrolix.required_dictionaries.length} required dictionar(y/ies)...`);
      
      for (const dictionaryName of bundle.dependencies.hydrolix.required_dictionaries) {
        try {
          await hdx.checkAndCreateDictionary(bearerToken, dictionaryName, base);
        } catch (e) {
          console.warn(`⚠️  WARNING: Failed to create dictionary ${dictionaryName}: ${getErrorMessage(e)}`);
          console.warn(`⚠️  Continuing anyway - transforms may be invalid until dictionary is added`);
        }
      }
    }
  }

  const datalink = await grafana.createDatalink(projectName);

  output.cluster_domain = BUNDLE_TESTING_CLUSTER;
  output.project_name = projectName;
  output.grafana_domain = `${GRAFANA_LOCATION}/`;
  output.datalink = datalink;

  let dashboardData = await loadDashboardTemplate(base, bundle, projectName, datalink);

  // Create base tables and transformations
  for (const table of bundle.tables) {
    dashboardData = dashboardData.replace(table.dashboard_var, table.name);
    await processTable(base, bearerToken, projectName, table, output, bundle);
  }

  // Create summary tables if present
  if (bundle.summary_tables) {
    for (const summary of bundle.summary_tables) {
      await createSummaryTable(
        base,
        bearerToken,
        projectName,
        summary,
        dashboardData
      );
      const fullTableName = `${projectName}.${summary.name}`;
      console.log(`Replacing ${summary.dashboard_var} with ${fullTableName}`);
      dashboardData = dashboardData.replace(summary.dashboard_var, summary.name);
    }
  }

  // Second pass: insert data into base tables to populate summaries
  await seedTablesWithData(base, bearerToken, projectName, bundle);

  // Create Grafana dashboard
  const dashboardId = await grafana.createDashboard(dashboardData);

  output.dashboard_id = dashboardId;

  // Create other dashboards if present
  if (bundle.other_dashboards) {
    for (const otherDash of bundle.other_dashboards) {
      console.log(`Creating additional dashboard: ${otherDash.path}`);

      let otherDashboardData = await Deno.readTextFile(`${base}/${otherDash.path}`);

      otherDashboardData = otherDashboardData.replace(/__PROJECT_NAME__/g, projectName);
      otherDashboardData = otherDashboardData.replace(/__DATASOURCE__/g, datalink);
      otherDashboardData = otherDashboardData.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());

      if (bundle.summary_tables && bundle.summary_tables.length > 0) {
        otherDashboardData = otherDashboardData.replace(/\$\{?VAR_SUMMARY_MIN\}?/g, `${projectName}.${bundle.summary_tables[0].name}`);
      }
      if (bundle.summary_tables && bundle.summary_tables.length > 1) {
        otherDashboardData = otherDashboardData.replace(/\$\{?VAR_SUMMARY_HOUR\}?/g, `${projectName}.${bundle.summary_tables[1].name}`);
      }

      for (const table of bundle.tables) {
        otherDashboardData = otherDashboardData.replace(new RegExp(table.dashboard_var, 'g'), table.name);
      }

      if (bundle.summary_tables) {
        for (const summary of bundle.summary_tables) {
          otherDashboardData = otherDashboardData.replace(new RegExp(summary.dashboard_var, 'g'), summary.name);
        }
      }

      await grafana.createDashboard(otherDashboardData);
      console.log(`✓ Created dashboard: ${otherDash.path}`);
    }
  }

  // Create alert rules if present
  if (bundle.alert_rules) {
    await createAlertRules(base, bundle, projectName, datalink, dashboardId);
  }

  return dashboardId;
}

async function loadDashboardTemplate(
  base: string,
  bundle: Bundle,
  projectName: string,
  datalink: string
): Promise<string> {
  const path = `${base}/${bundle.dashboard.path}`;

  let dashboard = await Deno.readTextFile(path);

  dashboard = dashboard.replace(/__PROJECT_NAME__/g, projectName);
  dashboard = dashboard.replace(/__DATASOURCE__/g, datalink);
  dashboard = dashboard.replace(/__DASHBOARD_UUID__/g, crypto.randomUUID());

  if (bundle.summary_tables && bundle.summary_tables.length > 0) {
    dashboard = dashboard.replace(/\$\{?VAR_SUMMARY_MIN\}?/g, `${projectName}.${bundle.summary_tables[0].name}`);
  }
  if (bundle.summary_tables && bundle.summary_tables.length > 1) {
    dashboard = dashboard.replace(/\$\{?VAR_SUMMARY_HOUR\}?/g, `${projectName}.${bundle.summary_tables[1].name}`);
  }

  return dashboard;
}

async function processTable(
  base: string,
  bearerToken: string,
  projectName: string,
  table: { name: string; dashboard_var: string; transforms: Array<{ path: string; sha256?: string; sample?: string }> },
  output: Output,
  bundle: Bundle
): Promise<void> {
  console.log(`Creating table: ${table.name}`);

  const tableUuid = await hdx.createTable(bearerToken, table.name);

  console.log("Waiting for table to be ready...");
  await new Promise(resolve => setTimeout(resolve, TABLE_READY_DELAY_SECS * 1000));

  const outputTable: OutputTable = {
    table_name: table.name,
    transforms: [],
  };

  for (const transform of table.transforms) {
    let transformJson = await readTransformFile(base, transform.path);

    // Replace __PROJECT_NAME__ in transform SQL
    transformJson = replaceFunctionNames(transformJson, projectName, bundle);

    const transformName = await addTransformation(
      bearerToken,
      tableUuid,
      transformJson,
      table.name,
      projectName,
      transform.path
    );

    await insertSampleDataIfPresent(
      bearerToken,
      projectName,
      table.name,
      transformName,
      transformJson
    );

    outputTable.transforms.push({
      name: transformName,
      data_type: getTransformationType(transformJson),
      data_sub_type: getTransformationSubtype(transformJson),
    });
  }

  output.tables.push(outputTable);
}

async function createSummaryTable(
  base: string,
  bearerToken: string,
  projectName: string,
  summary: { name: string; parent_table_name: string; dashboard_var: string; sql: { path: string } },
  dashboardData: string
): Promise<void> {
  const path = `${base}/${summary.sql.path}`;

  let sql = await Deno.readTextFile(path);

  sql = sql.replace(/__PROJECT_NAME__/g, projectName);
  sql = sql.replace(/__TABLE_NAME__/g, summary.parent_table_name);

  await hdx.createSummaryTable(bearerToken, summary.name, sql);
}

async function seedTablesWithData(
  base: string,
  bearerToken: string,
  projectName: string,
  bundle: Bundle
): Promise<void> {
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      let transformJson = await readTransformFile(base, transform.path);

      // Replace __PROJECT_NAME__ in transform SQL
      transformJson = replaceFunctionNames(transformJson, projectName, bundle);

      const transformName = getTransformationName(transformJson);

      await insertSampleDataIfPresent(
        bearerToken,
        projectName,
        table.name,
        transformName,
        transformJson
      );
    }
  }
}

async function readTransformFile(base: string, relativePath: string): Promise<unknown> {
  const path = `${base}/${relativePath}`;

  try {
    const content = await Deno.readTextFile(path);
    return JSON.parse(content);
  } catch (e) {
    throw new Error(`Failed to read/parse transform path=${path}: ${getErrorMessage(e)}`);
  }
}

async function addTransformation(
  bearerToken: string,
  tableUuid: string,
  transformJson: unknown,
  tableName: string,
  projectName: string,
  transformPath: string
): Promise<string> {
  const fullTableName = `${projectName}.${tableName}`;

  try {
    return await hdx.addTransformToTable(bearerToken, tableUuid, transformJson);
  } catch (e) {
    throw new Error(
      `Failed to add transformation path=${transformPath} table=${fullTableName}: ${getErrorMessage(e)}`
    );
  }
}

async function insertSampleDataIfPresent(
  bearerToken: string,
  projectName: string,
  tableName: string,
  transformName: string,
  transformJson: unknown
): Promise<void> {
  const sampleData = getSampleDataAsJson(transformJson);

  if (!sampleData) {
    console.log(`ℹ No sample data found for transform ${transformName}, skipping insertion`);
    return;
  }

  console.log(`Found sample data for transform ${transformName}, preparing to insert...`);
  console.log("Waiting for table to be ready for data...");
  await new Promise(resolve => setTimeout(resolve, DATA_READY_DELAY_SECS * 1000));

  const fullTableName = `${projectName}.${tableName}`;

  console.log(`Inserting sample data into ${fullTableName} with transform ${transformName}...`);

  try {
    await hdx.insertIntoTable(bearerToken, fullTableName, transformName, sampleData);
    console.log(`✓ Successfully inserted sample data into ${fullTableName}`);
  } catch (e) {
    console.warn(`⚠️  WARNING: Failed to insert data into ${fullTableName}: ${getErrorMessage(e)}`);
    console.warn(`⚠️  Continuing with deployment anyway - dashboard will be created without data`);
  }
}

function getTransformationSubtype(transformJson: unknown): string {
  const data = transformJson as Record<string, unknown>;
  const settings = data.settings as Record<string, unknown> | undefined;
  const formatDetails = settings?.format_details as Record<string, unknown> | undefined;
  const subtype = formatDetails?.subtype;
  return typeof subtype === 'string' ? subtype : '';
}

function getTransformationType(transformJson: unknown): string {
  const data = transformJson as Record<string, unknown>;
  const type = data.type;
  return typeof type === 'string' ? type : '';
}

function getSampleDataAsJson(transformJson: unknown): unknown | null {
  const data = transformJson as Record<string, unknown>;
  const settings = data.settings as Record<string, unknown> | undefined;
  const sampleData = settings?.sample_data;

  if (sampleData && typeof sampleData === 'object' && Object.keys(sampleData).length > 0) {
    return sampleData;
  }

  return null;
}

function getTransformationName(transformJson: unknown): string {
  const data = transformJson as Record<string, unknown>;
  const name = data.name;
  return typeof name === 'string' ? name : '';
}

function replaceFunctionNames(transformJson: unknown, projectName: string, bundle: Bundle): unknown {
  const data = transformJson as Record<string, unknown>;
  const settings = data.settings as Record<string, unknown> | undefined;
  const sqlTransform = settings?.sql_transform;

  if (!sqlTransform || typeof sqlTransform !== 'string') {
    return transformJson;
  }

  // Simply replace __PROJECT_NAME__ macro with actual project name
  const updatedSql = sqlTransform.replace(/__PROJECT_NAME__/g, projectName);

  return {
    ...data,
    settings: {
      ...(settings || {}),
      sql_transform: updatedSql,
    },
  };
}

async function createAlertRules(
  base: string,
  bundle: Bundle,
  projectName: string,
  datalink: string,
  dashboardId: string
): Promise<void> {
  if (!bundle.alert_rules) {
    return;
  }

  console.log("Loading and processing alert rules...");

  const alertRulesPath = `${base}/${bundle.alert_rules.path}`;
  let alertRulesContent = await Deno.readTextFile(alertRulesPath);

  alertRulesContent = alertRulesContent.replace(/__PROJECT_NAME__/g, projectName);
  alertRulesContent = alertRulesContent.replace(/__DATASOURCE__/g, datalink);
  alertRulesContent = alertRulesContent.replace(/__DASHBOARD_UUID__/g, dashboardId);

  for (const table of bundle.tables) {
    const fullTableName = `${projectName}.${table.name}`;
    alertRulesContent = alertRulesContent.replace(new RegExp(table.dashboard_var, 'g'), fullTableName);
  }

  if (bundle.summary_tables) {
    for (const summary of bundle.summary_tables) {
      const fullTableName = `${projectName}.${summary.name}`;
      alertRulesContent = alertRulesContent.replace(new RegExp(summary.dashboard_var, 'g'), fullTableName);
    }
  }

  await grafana.createAlertRules(alertRulesContent);
}