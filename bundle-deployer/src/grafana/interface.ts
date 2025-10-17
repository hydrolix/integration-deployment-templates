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
  port: string;
  server: string;
  query_timeout: string;
  secure: boolean;
  timeout: string;
  username: string;
}

interface SecureJsonData {
  password: string;
}

export async function createDatalink(projectName: string): Promise<string> {
  const datasourceRequest: CreateDataSourceRequest = {
    name: "Bundle Testing",
    type: "grafana-clickhouse-datasource",
    access: "proxy",
    jsonData: {
      default_database: projectName,
      port: HDX_DATABASE_PORT,
      server: BUNDLE_TESTING_CLUSTER,
      query_timeout: "600",
      secure: true,
      timeout: "10",
      username: BUNDLE_TESTING_USERNAME,
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