// Hydrolix API client - Complete implementation with YAML/Regexp dictionary support

import { getErrorMessage } from "./utils/error.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const BUNDLE_TESTING_USERNAME = Deno.env.get("BUNDLE_TESTING_USERNAME") || "";
const BUNDLE_TESTING_PASSWORD = Deno.env.get("BUNDLE_TESTING_PASSWORD") || "";
const FOR_MARKETPLACE = Deno.args.includes("--marketplace");

// These are static but not secret
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";

const ORG_UUID_PROD = "a3583b75-5042-44a0-8198-54fca9f2f187";
const PROJ_UUID_PROD = "fcd20095-8458-49c1-9506-5951a614f49b";
const PROJ_NAME = "sample_project";


const HTTP_TIMEOUT = 120000; // 

// ============================================================================
// AUTH
// ============================================================================

export async function getAuthToken(): Promise<string> {
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
    throw new Error(`Failed to get auth token from ${url}: ${getErrorMessage(e)}`);
  }
}

export function createProjectName(): string {
  return PROJ_NAME;
}

// ============================================================================
// FUNCTIONS
// ============================================================================

export async function checkAndCreateFunction(
  bearerToken: string,
  functionName: string,
  baseDir: string
): Promise<void> {
  console.log(`Checking function: ${functionName}...`);
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
  const expectedName = `${PROJ_NAME}_${functionName}`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (listResponse.ok) {
      const responseData = await listResponse.json();
      
      let existing: Array<{ name: string }> = [];
      if (Array.isArray(responseData)) {
        existing = responseData;
      } else if (responseData?.functions && Array.isArray(responseData.functions)) {
        existing = responseData.functions;
      } else if (responseData?.data && Array.isArray(responseData.data)) {
        existing = responseData.data;
      }
      
      if (existing.some(f => f.name === expectedName)) {
        console.log(`  ✓ Function ${functionName} already exists (as ${expectedName})`);
        return;
      }
    }
  } catch (e) {
    console.warn(`  ⚠️  Could not check for existing function: ${getErrorMessage(e)}`);
  }
  
  const functionFilePath = `${baseDir}/functions/${functionName}.json`;
  
  try {
    await Deno.stat(functionFilePath);
  } catch {
    console.warn(`  ⚠️  WARNING: Function ${functionName} not found on cluster and no local file: ${functionFilePath}`);
    console.warn(`  ⚠️  Transforms may fail if they reference this function`);
    return;
  }
  
  let functionDef;
  try {
    const content = await Deno.readTextFile(functionFilePath);
    functionDef = JSON.parse(content);
  } catch (e) {
    throw new Error(`Failed to read function file ${functionFilePath}: ${getErrorMessage(e)}`);
  }
  
  // Replace __PROJECT_NAME__ in function SQL
  if (functionDef.sql && typeof functionDef.sql === 'string') {
    functionDef.sql = functionDef.sql.replace(/__PROJECT_NAME__/g, PROJ_NAME);
  }
  
  const createUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    console.log(`  Creating function ${functionName} (will become ${expectedName})...`);
    
    const response = await fetch(createUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify({
        ...functionDef,
        name: functionName,
      }),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`HTTP ${response.status}: ${errorText}`);
    }
    
    console.log(`  ✓ Created function ${functionName}`);
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create function ${functionName}: ${getErrorMessage(e)}`);
  }
}

// ============================================================================
// DICTIONARIES
// ============================================================================

export async function checkAndCreateDictionary(
  bearerToken: string,
  dictionaryName: string,
  baseDir: string
): Promise<void> {
  console.log(`Checking dictionary: ${dictionaryName}...`);
  
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  const expectedName = `${PROJ_NAME}_${dictionaryName}`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (listResponse.ok) {
      const responseData = await listResponse.json();
      
      let existing: Array<{ name: string }> = [];
      if (Array.isArray(responseData)) {
        existing = responseData;
      } else if (responseData?.dictionaries && Array.isArray(responseData.dictionaries)) {
        existing = responseData.dictionaries;
      } else if (responseData?.data && Array.isArray(responseData.data)) {
        existing = responseData.data;
      }
      
      if (existing.some(d => d.name === expectedName)) {
        console.log(`  ✓ Dictionary ${dictionaryName} already exists (as ${expectedName})`);
        return;
      }
    }
  } catch (e) {
    console.warn(`  ⚠️  Could not check for existing dictionary: ${getErrorMessage(e)}`);
  }
  
  const jsonFilePath = `${baseDir}/dictionaries/${dictionaryName}.json`;
  
  try {
    await Deno.stat(jsonFilePath);
  } catch {
    console.warn(`  ⚠️  WARNING: Dictionary ${dictionaryName} not found on cluster and no local file: ${jsonFilePath}`);
    console.warn(`  ⚠️  Transforms may fail if they reference this dictionary`);
    return;
  }
  
  let dictDef;
  try {
    const content = await Deno.readTextFile(jsonFilePath);
    dictDef = JSON.parse(content);
  } catch (e) {
    throw new Error(`Failed to read dictionary definition ${jsonFilePath}: ${getErrorMessage(e)}`);
  }
  
  const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
  let dataFilePath = '';
  let dataFileContent = '';
  
  for (const ext of possibleExtensions) {
    const path = `${baseDir}/dictionaries/${dictionaryName}.${ext}`;
    try {
      await Deno.stat(path);
      dataFilePath = path;
      dataFileContent = await Deno.readTextFile(path);
      console.log(`  Found data file: dictionaries/${dictionaryName}.${ext}`);
      break;
    } catch {
      // Try next
    }
  }
  
  if (!dataFilePath) {
    throw new Error(
      `Dictionary ${dictionaryName} has definition file but no data file ` +
      `(checked: ${possibleExtensions.map(e => `.${e}`).join(', ')})`
    );
  }
  
  const fileName = dataFilePath.split('/').pop()!;
  await uploadDictionaryFile(bearerToken, fileName, dataFileContent);
  await createDictionaryDefinition(bearerToken, dictionaryName, dictDef);
  
  console.log(`  ✓ Created dictionary ${dictionaryName}`);
}

async function uploadDictionaryFile(
  bearerToken: string,
  fileName: string,
  fileContent: string
): Promise<void> {
  const filesUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/files/`;
  
  try {
    const filesListResponse = await fetch(filesUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (filesListResponse.ok) {
      const existingFiles = await filesListResponse.json();
      
      if (Array.isArray(existingFiles)) {
        const fileExists = existingFiles.some((f: any) => 
          typeof f === 'string' ? f === fileName : f.name === fileName
        );
        
        if (fileExists) {
          console.log(`  ✓ Dictionary file already uploaded: ${fileName}`);
          return;
        }
      }
    }
  } catch (e) {
    console.warn(`  ⚠️  Could not check for existing files: ${getErrorMessage(e)}`);
  }
  
  const ext = fileName.split('.').pop()?.toLowerCase();
  const mimeType = ext === 'yaml' || ext === 'yml' ? 'application/x-yaml' : 'text/csv';
  
  const formData = new FormData();
  formData.append('file', new Blob([fileContent], { type: mimeType }), fileName);
  formData.append('name', fileName);
  
  try {
    console.log(`  Uploading dictionary file: ${fileName}...`);
    
    const uploadResponse = await fetch(filesUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
      },
      body: formData,
    });
    
    if (!uploadResponse.ok) {
      const errorText = await uploadResponse.text();
      throw new Error(`Failed to upload: ${errorText}`);
    }
    
    console.log(`  ✓ Uploaded dictionary file: ${fileName}`);
  } catch (e) {
    throw new Error(`Failed to upload dictionary file: ${getErrorMessage(e)}`);
  }
}

async function createDictionaryDefinition(
  bearerToken: string,
  dictionaryName: string,
  dictDefinition: any
): Promise<void> {
  const dictUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  const expectedName = `${PROJ_NAME}_${dictionaryName}`;
  
  const payload = {
    ...dictDefinition,
    name: dictionaryName,
  };
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    console.log(`  Creating dictionary definition: ${dictionaryName} (will become ${expectedName})...`);
    
    const dictResponse = await fetch(dictUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!dictResponse.ok) {
      const errorText = await dictResponse.text();
      throw new Error(`HTTP ${dictResponse.status}: ${errorText}`);
    }
    
    console.log(`  ✓ Created dictionary definition`);
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create dictionary definition: ${getErrorMessage(e)}`);
  }
}

// ============================================================================
// TABLES
// ============================================================================

export async function createTable(bearerToken: string, tableName: string): Promise<string> {
  const payload = FOR_MARKETPLACE ? {
    name: tableName,
    description: "testing",
    settings: {
      age: {
        max_age_days: 1,
      },
      merge: {
        enabled: false,
      },
      default_query_options: {
        hdx_query_max_timerange_sec: 2592000,
        hdx_query_max_result_rows: 5000000,
        hdx_query_max_execution_time: 180,
      },
    },
  } : {
    name: tableName,
    description: "testing",
    settings: {
      age: {
        max_age_days: 1,
      },
      merge: {
        enabled: false,
      },
    },
  };
  
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/plain, */*',
      },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const tableData = await response.json();
    const uuid = tableData?.uuid;
    
    if (!uuid) {
      throw new Error("table UUID not found in response");
    }
    
    return uuid;
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create table at ${url}: ${getErrorMessage(e)}`);
  }
}

export async function getTableList(bearerToken: string, debugMode = false): Promise<string> {
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables`;
  
  if (debugMode) {
    console.log("DEBUG: Hdx listing tables...");
  }
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/plain, */*',
      },
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    return await response.text();
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to list tables: ${getErrorMessage(e)}`);
  }
}

export async function deleteTable(bearerToken: string, uuid: string): Promise<void> {
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables/${uuid}`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/plain, */*',
      },
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to delete table: ${getErrorMessage(e)}`);
  }
}

// ============================================================================
// SUMMARY TABLES
// ============================================================================

export async function createSummaryTable(
  bearerToken: string,
  tableName: string,
  sql: string
): Promise<string> {
  const payload = {
    name: tableName,
    type: "summary",
    settings: {
      summary: {
        enabled: true,
        sql: sql,
      },
    },
  };
  
  console.error(`payload=${JSON.stringify(payload, null, 2)}`);
  
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json, text/plain, */*',
      },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    return tableName;
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create summary table at ${url}: ${getErrorMessage(e)}`);
  }
}

// ============================================================================
// TRANSFORMS
// ============================================================================

export async function addTransformToTable(
  bearerToken: string,
  tableUuid: string,
  transformJson: unknown
): Promise<string> {
  const transformData = transformJson as Record<string, unknown>;
  const transformName = transformData.name as string;
  
  if (!transformName) {
    throw new Error(`Could not find the transformation name in ${JSON.stringify(transformJson)}`);
  }
  
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/tables/${tableUuid}/transforms/`;
  
  const maxRetries = 5;
  const baseDelay = 1000;
  const maxDelay = 30000;
  
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
    
    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${bearerToken}`,
          'Content-Type': 'application/json',
          'Accept': 'application/json',
        },
        body: JSON.stringify(transformJson),
        signal: controller.signal,
      });
      
      clearTimeout(timeoutId);
      
      if (response.ok) {
        return transformName;
      }
      
      if (response.status >= 500 && attempt <= maxRetries) {
        const delay = calculateBackoff(attempt, baseDelay, maxDelay);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      
      throw new Error(
        `Hydrolix add transform failed, status: ${response.status} (attempt ${attempt})`
      );
      
    } catch (e) {
      clearTimeout(timeoutId);
      
      if (attempt >= maxRetries) {
        throw new Error(
          `Failed to add transform after ${attempt} attempts: ${getErrorMessage(e)}`
        );
      }
      
      const delay = calculateBackoff(attempt, baseDelay, maxDelay);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  
  throw new Error(`Failed to add transform after ${maxRetries} attempts`);
}

function calculateBackoff(attempt: number, baseDelay: number, maxDelay: number): number {
  const exponent = attempt - 1;
  const delayMs = baseDelay * Math.pow(2, exponent);
  return Math.min(delayMs, maxDelay);
}

// ============================================================================
// DATA INSERTION
// ============================================================================

export async function insertIntoTable(
  bearerToken: string,
  fullTableName: string,
  transformName: string,
  sampleData: unknown
): Promise<void> {
  const payload = Array.isArray(sampleData) ? sampleData : [sampleData];
  
  const url = `https://${BUNDLE_TESTING_CLUSTER}/ingest/event`;
  
  const maxRetries = 20;
  const baseDelayMs = 1000;
  const maxDelayMs = 60000;
  const backoffFactor = 2.0;
  
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    if (attempt > 0) {
      const exponentialDelay = baseDelayMs * Math.pow(backoffFactor, attempt - 1);
      const finalDelay = Math.min(exponentialDelay, maxDelayMs);
      
      console.log(`Retry attempt ${attempt + 1}/${maxRetries}, waiting ${finalDelay}ms`);
      await new Promise(resolve => setTimeout(resolve, finalDelay));
    }
    
    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${bearerToken}`,
          'Content-Type': 'application/json',
          'Accept': 'application/json',
          'x-hdx-table': fullTableName,
          'x-hdx-transform': transformName,
        },
        body: JSON.stringify(payload),
      });
      
      if (response.ok) {
        if (attempt > 0) {
          console.log(`Successfully inserted data after ${attempt + 1} retries`);
        }
        return;
      }
      
      const status = response.status;
      const errorBody = await response.text();
      
      const isRetryable = 
        (status >= 500 && status <= 599) ||
        status === 408 ||
        status === 429;
      
      console.error(
        `Hydrolix insert failed on attempt ${attempt + 1}/${maxRetries}, ` +
        `status: ${status} (retryable: ${isRetryable}) url=${url}`
      );
      console.error(`Error response body: ${errorBody}`);
      
      if (!isRetryable && status >= 400 && status < 500) {
        throw new Error(
          `Non-retryable error ${status} for ${fullTableName}: ${errorBody}`
        );
      }
      
    } catch (e) {
      console.error(`Request error on attempt ${attempt + 1}: ${getErrorMessage(e)}`);
      
      const errorMsg = getErrorMessage(e);
      if (errorMsg.includes('Non-retryable')) {
        throw e;
      }
      
      if (attempt === maxRetries - 1) {
        throw new Error(
          `Failed to send data to ${fullTableName} after ${maxRetries} attempts: ${errorMsg}`
        );
      }
    }
  }
  
  throw new Error(
    `Failed to send data to ${fullTableName} after ${maxRetries} attempts`
  );
}

export function createTableName(): string {
  const uuid = crypto.randomUUID();
  const ending = uuid.replace(/-/g, '').slice(0, 10);
  return `testing_${ending}`;
}