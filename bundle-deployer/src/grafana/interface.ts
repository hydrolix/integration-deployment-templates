// Grafana API client - Complete implementation

import { GRAFANA_LOCATION } from "./container.ts";
import { getErrorMessage } from "../utils/error.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const BUNDLE_TESTING_USERNAME = Deno.env.get("BUNDLE_TESTING_USERNAME") || "";
const BUNDLE_TESTING_PASSWORD = Deno.env.get("BUNDLE_TESTING_PASSWORD") || "";

const HDX_DATABASE_PORT = "9440";
const HTTP_TIMEOUT = 120000; // 120 seconds in milliseconds

interface CreateDataSourceRequest {
  name: string;
  type: string;
  access: string;
  jsonData: JsonData;
  secureJsonData: SecureJsonData;
  readOnly: boolean;
}

interface JsonData {
  default_database: string;
  host: string;
  port: number;
  protocol: string;
  query_timeout: string;
  secure: boolean;
  timeout: string;
  username: string;
  defaultRound?: string;
  adhocVariableName?: string;
  useDefaultPort?: boolean;
  useDefaultDatabase?: boolean;
}

interface SecureJsonData {
  password: string;
}

interface AlertRulesFile {
  apiVersion: number;
  groups: Array<{
    orgId?: number;
    name: string;
    folder: string;
    interval: string;
    rules: Array<{
      uid: string;
      title: string;
      condition: string;
      data: unknown[];
      [key: string]: unknown;
    }>;
  }>;
}

export async function createDatalink(projectName: string): Promise<string> {
  console.log(`Creating Grafana datasource for project ${projectName}...`); 

  const datasourceRequest: CreateDataSourceRequest = {
    name: "Bundle Testing",
    type: "hydrolix-hydrolix-datasource",
    access: "proxy",
    jsonData: {
      default_database: projectName,
      host: BUNDLE_TESTING_CLUSTER,
      port: 9440,
      useDefaultPort: true,
      protocol: "native",
      query_timeout: "600",
      secure: true,
      timeout: "10",
      username: BUNDLE_TESTING_USERNAME,
      defaultRound: "1m",
      adhocVariableName: "table",
      useDefaultDatabase: true,
    },
    secureJsonData: {
      password: BUNDLE_TESTING_PASSWORD,
    },
    readOnly: true,
  };
  
  const url = `http://${GRAFANA_LOCATION}/api/datasources`;
  
  const response = await postBasicAuth(
    url,
    "admin",
    "admin",
    datasourceRequest,
    { "X-Grafana-Org-Id": "1" }
  );
  
  const responseJson = JSON.parse(response);
  const uid = responseJson?.datasource?.uid;
  
  if (!uid) {
    throw new Error("Failed to create Grafana Datalink - no UID in response");
  }

  console.log(`✓ Created Grafana datasource with UID: ${uid}`);
  
  // Wait a bit for Grafana to settle
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  return uid;
}

export async function createDashboard(dashboardData: string): Promise<string> {
  const url = `http://${GRAFANA_LOCATION}/api/dashboards/import`;
  
  const resultData = await postStringBasicAuth(
    url,
    "admin",
    "admin",
    dashboardData,
    { "X-Grafana-Org-Id": "1" }
  );
  
  const resultJson = JSON.parse(resultData);
  const uid = resultJson?.uid;
  
  if (!uid) {
    throw new Error("No UID in the dashboard response");
  }
  
  return uid;
}

export async function createAlertRules(alertRulesJson: string): Promise<void> {
  const alertRules = JSON.parse(alertRulesJson) as AlertRulesFile;
  
  console.log(`Creating ${alertRules.groups.length} alert rule group(s)...`);
  
  // Create folders and individual rules
  for (const group of alertRules.groups) {
    // First, ensure the folder exists
    const folderUid = await ensureFolderExists(group.folder);
    
    // Create each rule individually using the alert-rules endpoint
    await createRulesIndividually(folderUid, group);
  }
  
  console.log("✓ Successfully created all alert rules");
}

async function ensureFolderExists(folderTitle: string): Promise<string> {
  // Check if folder already exists
  const searchUrl = `http://${GRAFANA_LOCATION}/api/folders`;
  
  const searchResponse = await fetch(searchUrl, {
    method: 'GET',
    headers: {
      'Authorization': 'Basic ' + btoa('admin:admin'),
    },
  });
  
  if (searchResponse.ok) {
    const folders = await searchResponse.json() as Array<{ uid: string; title: string }>;
    const existingFolder = folders.find(f => f.title === folderTitle);
    
    if (existingFolder) {
      console.log(`  ✓ Folder "${folderTitle}" already exists (uid: ${existingFolder.uid})`);
      return existingFolder.uid;
    }
  }
  
  // Create new folder
  console.log(`  Creating folder "${folderTitle}"...`);
  
  const createUrl = `http://${GRAFANA_LOCATION}/api/folders`;
  
  const payload = {
    title: folderTitle,
  };
  
  const response = await fetch(createUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Basic ' + btoa('admin:admin'),
    },
    body: JSON.stringify(payload),
  });
  
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Failed to create folder "${folderTitle}": ${errorText}`);
  }
  
  const result = await response.json();
  const uid = result?.uid;
  
  if (!uid) {
    throw new Error(`Could not find folder UID in response for "${folderTitle}"`);
  }
  
  console.log(`  ✓ Created folder "${folderTitle}" (uid: ${uid})`);
  return uid;
}

async function createRulesIndividually(
  folderUid: string,
  group: AlertRulesFile['groups'][0]
): Promise<void> {
  console.log(`  Creating rule group "${group.name}" with ${group.rules.length} rule(s)...`);
  
  const url = `http://${GRAFANA_LOCATION}/api/v1/provisioning/alert-rules`;
  
  // Create each rule individually
  for (const rule of group.rules) {
    console.log(`    Creating rule "${rule.title}"...`);
    
    // Clean the rule - remove UI-only fields
    const { notification_settings, isPaused, templating, ...cleanRule } = rule as any;
    
    // Clean up data queries
    if (cleanRule.data && Array.isArray(cleanRule.data)) {
      cleanRule.data = cleanRule.data.map((query: any) => {
        if (query.model) {
          const { 
            meta, 
            pluginVersion, 
            format, 
            editorType,
            builderOptions,
            ...cleanModel 
          } = query.model;
          query.model = cleanModel;
        }
        return query;
      });
    }
    
    // Clean up annotations
    if (cleanRule.annotations) {
      const { __dashboardUid__, __panelId__, ...cleanAnnotations } = cleanRule.annotations;
      cleanRule.annotations = cleanAnnotations;
    }
    
    // Format payload for individual rule creation
    const payload = {
      folderUID: folderUid,
      ruleGroup: group.name,
      ...cleanRule,
    };
    
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Basic ' + btoa('admin:admin'),
        'X-Disable-Provenance': 'true',
      },
      body: JSON.stringify(payload),
    });
    
    if (!response.ok) {
      const errorText = await response.text();
      console.error(`    ERROR: Response status ${response.status}`);
      console.error(`    ERROR: Response body: ${errorText}`);
      console.error(`    ERROR: Payload sent: ${JSON.stringify(payload, null, 2)}`);
      throw new Error(`Failed to create alert rule "${rule.title}": ${errorText}`);
    }
    
    const result = await response.json();
    console.log(`    ✓ Created rule "${rule.title}" (id: ${result.id})`);
  }
  
  console.log(`  ✓ Created rule group "${group.name}"`);
}

async function postBasicAuth(
  url: string,
  username: string,
  password: string,
  payload: unknown,
  additionalHeaders?: Record<string, string>
): Promise<string> {
  const headers: Record<string, string> = {
    'Accept': 'application/json',
    'Content-Type': 'application/json',
    'Authorization': 'Basic ' + btoa(`${username}:${password}`),
  };
  
  // Add additional headers if provided
  if (additionalHeaders) {
    Object.assign(headers, additionalHeaders);
  }
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    const text = await response.text();
    
    if (!response.ok) {
      throw new Error(
        `Failed post url=${url} status_code=${response.status} text=${text} ` +
        `payload=${JSON.stringify(payload)}`
      );
    }
    
    return text;
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to post to ${url}: ${getErrorMessage(e)}`);
  }
}

async function postStringBasicAuth(
  url: string,
  username: string,
  password: string,
  payload: string,
  additionalHeaders?: Record<string, string>
): Promise<string> {
  const headers: Record<string, string> = {
    'Accept': 'application/json',
    'Content-Type': 'application/json',
    'Authorization': 'Basic ' + btoa(`${username}:${password}`),
  };
  
  // Add additional headers if provided
  if (additionalHeaders) {
    Object.assign(headers, additionalHeaders);
  }
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: payload,
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    const text = await response.text();
    
    if (!response.ok) {
      throw new Error(
        `Failed post url=${url} status_code=${response.status} text=${text}`
      );
    }
    
    return text;
    } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to post to ${url}: ${getErrorMessage(e)}`);
  }
}