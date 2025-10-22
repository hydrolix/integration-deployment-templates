// Hydrolix API client - Complete implementation with YAML/Regexp dictionary support

import { getErrorMessage } from "./utils/error.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const BUNDLE_TESTING_USERNAME = Deno.env.get("BUNDLE_TESTING_USERNAME") || "";
const BUNDLE_TESTING_PASSWORD = Deno.env.get("BUNDLE_TESTING_PASSWORD") || "";
const FOR_MARKETPLACE = Deno.args.includes("--marketplace");

// These are static but not secret
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";
const PROJ_NAME = "sample_project";

const HTTP_TIMEOUT = 120000; // 120 seconds in milliseconds

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

export async function createFunctions(
  bearerToken: string,
  functions: Array<{ name: string; description?: string; sql: string }>,
  forceRecreate = false
): Promise<void> {
  if (functions.length === 0) {
    return;
  }
  
  console.log(`Checking/creating ${functions.length} custom function(s)...`);
  
  // Get existing functions first
  const existingFunctionsUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
  
  let existingFunctions: Map<string, string>;
  try {
    const listResponse = await fetch(existingFunctionsUrl, {
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
      },
    });
    
    if (listResponse.ok) {
      const existingList = await listResponse.json() as Array<{ name: string; uuid?: string }>;
      existingFunctions = new Map(existingList.map(f => [f.name, f.uuid || '']));
    } else {
      existingFunctions = new Map();
    }
  } catch {
    existingFunctions = new Map();
  }
  
  // Check which functions need to be created
  const functionsToCreate = [];
  
  for (const fn of functions) {
    const fullName = `${PROJ_NAME}_${fn.name}`;
    
    if (existingFunctions.has(fullName)) {
      if (forceRecreate) {
        // Delete the existing function first
        const uuid = existingFunctions.get(fullName);
        if (uuid) {
          console.log(`  Deleting existing function ${fn.name} to recreate...`);
          await deleteFunction(bearerToken, uuid);
        }
        functionsToCreate.push(fn);
      } else {
        console.log(`  ✓ Function ${fn.name} already exists (as ${fullName})`);
      }
    } else {
      functionsToCreate.push(fn);
    }
  }
  
  if (functionsToCreate.length === 0) {
    console.log(`✓ All functions already exist`);
    return;
  }
  
  console.log(`Creating ${functionsToCreate.length} new function(s)...`);
  
  // Create functions one at a time (not bulk) to avoid batch failures
  let successCount = 0;
  let failCount = 0;
  
  for (const fn of functionsToCreate) {
    const prefixedName = `${PROJ_NAME}_${fn.name}`;
    const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
    
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
    
    try {
      console.log(`  Creating function ${prefixedName}...`);
      
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${bearerToken}`,
          'Content-Type': 'application/json',
          'Accept': 'application/json',
        },
        body: JSON.stringify({
          ...fn,
          name: prefixedName,
        }),
        signal: controller.signal,
      });
      
      clearTimeout(timeoutId);
      
      if (!response.ok) {
        const errorText = await response.text();
        console.warn(`  ⚠️  Failed to create ${fn.name}: ${errorText}`);
        failCount++;
      } else {
        console.log(`  ✓ Created function ${fn.name}`);
        successCount++;
      }
    } catch (e) {
      clearTimeout(timeoutId);
      console.warn(`  ⚠️  Failed to create ${fn.name}: ${getErrorMessage(e)}`);
      failCount++;
    }
  }
  
  if (successCount > 0) {
    console.log(`✓ Successfully created ${successCount} function(s)`);
  }
  if (failCount > 0) {
    console.warn(`⚠️  Failed to create ${failCount} function(s)`);
  }
}

async function deleteFunction(bearerToken: string, uuid: string): Promise<void> {
  const url = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/${uuid}`;
  
  await fetch(url, {
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${bearerToken}`,
    },
  });
}

export async function createDictionary(
  bearerToken: string,
  dictionary: { name: string; source: string },
  baseDir?: string
): Promise<void> {
  console.log(`Checking/creating dictionary ${dictionary.name}...`);
  
  // Check if dictionary already exists
  const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  
  try {
    const listResponse = await fetch(listUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (listResponse.ok) {
      const existing = await listResponse.json() as Array<{ name: string }>;
      const fullName = `${PROJ_NAME}_${dictionary.name}`;
      
      if (existing.some(d => d.name === fullName)) {
        console.log(`  ✓ Dictionary ${dictionary.name} already exists (as ${fullName})`);
        return;
      }
    }
  } catch {
    // Continue to create if we can't check
  }
  
  // Determine file type
  const isYaml = dictionary.source.toLowerCase().endsWith('.yaml') || dictionary.source.toLowerCase().endsWith('.yml');
  const isJsonDefinition = dictionary.source.toLowerCase().endsWith('.json');
  const isUrl = dictionary.source.startsWith('http://') || dictionary.source.startsWith('https://');
  
  // If not a URL, check if local file exists
  if (!isUrl) {
    if (!baseDir) {
      console.warn(`  ⚠️ Dictionary ${dictionary.name} not found on cluster and no base directory provided`);
      console.warn(`  ⚠️ Expected to be pre-loaded by infrastructure team - skipping`);
      return;
    }
    
    const filePath = `${baseDir}/${dictionary.source}`;
    try {
      await Deno.stat(filePath);
    } catch {
      console.warn(`  ⚠️ Dictionary ${dictionary.name} not found on cluster`);
      console.warn(`  ⚠️ Local file not found: ${filePath}`);
      console.warn(`  ⚠️ Expected to be pre-loaded by infrastructure team - skipping`);
      return;
    }
  }
  
  // Load file content
  let fileContent: string;
  if (isUrl) {
    let sourceUrl = dictionary.source;
    if (sourceUrl.includes("__CLUSTER__")) {
      sourceUrl = sourceUrl.replace("__CLUSTER__", `https://${BUNDLE_TESTING_CLUSTER}`);
    }
    console.log(`Fetching dictionary file from ${sourceUrl}...`);
    const fileResponse = await fetch(sourceUrl);
    if (!fileResponse.ok) {
      throw new Error(`Failed to fetch dictionary file: ${fileResponse.statusText}`);
    }
    fileContent = await fileResponse.text();
  } else {
    const filePath = `${baseDir}/${dictionary.source}`;
    console.log(`Reading local dictionary file from ${filePath}...`);
    fileContent = await Deno.readTextFile(filePath);
  }
  
  // Handle JSON dictionary definitions (pre-configured payloads)
  if (isJsonDefinition) {
    await createDictionaryFromDefinition(bearerToken, dictionary.name, fileContent);
    return;
  }
  
  // Handle YAML and CSV files (need upload + definition)
  const fileName = dictionary.source.split('/').pop() || `${dictionary.name}.${isYaml ? 'yaml' : 'csv'}`;
  
  // Upload file
  await uploadDictionaryFile(bearerToken, fileName, fileContent, isYaml);
  
  // Create dictionary definition
  if (isYaml) {
    await createRegexpDictionary(bearerToken, dictionary.name, fileName, fileContent);
  } else {
    await createCsvDictionary(bearerToken, dictionary.name, fileName, fileContent);
  }
}

async function uploadDictionaryFile(
  bearerToken: string,
  fileName: string,
  fileContent: string,
  isYaml: boolean
): Promise<void> {
  const filesUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/files/`;
  
  // Check if file already exists
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
          console.log(`  ✓ Dictionary file ${fileName} already on server (skipping upload)`);
          return;
        }
      }
    }
  } catch (e) {
    console.warn(`  ⚠ Could not check for existing files: ${getErrorMessage(e)}`);
  }
  
  // Upload the file
  const mimeType = isYaml ? 'application/x-yaml' : 'text/csv';
  const formData = new FormData();
  formData.append('file', new Blob([fileContent], { type: mimeType }), fileName);
  formData.append('name', fileName);
  
  try {
    const uploadResponse = await fetch(filesUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
      },
      body: formData,
    });
    
    if (!uploadResponse.ok) {
      const errorText = await uploadResponse.text();
      throw new Error(`Failed to upload dictionary file: ${errorText}`);
    }
    
    console.log(`✓ Uploaded dictionary file ${fileName}`);
  } catch (e) {
    throw new Error(`Failed to upload dictionary file: ${getErrorMessage(e)}`);
  }
}

async function createDictionaryFromDefinition(
  bearerToken: string,
  dictName: string,
  jsonContent: string
): Promise<void> {
  console.log(`Creating dictionary from pre-configured definition...`);
  
  const definition = JSON.parse(jsonContent);
  
  // Override the name with project prefix
  definition.name = `${PROJ_NAME}_${dictName}`;
  
  const dictUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const dictResponse = await fetch(dictUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify(definition),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!dictResponse.ok) {
      const errorText = await dictResponse.text();
      throw new Error(`Failed to create dictionary from definition: ${errorText}`);
    }
    
    console.log(`✓ Successfully created dictionary ${dictName} from definition`);
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create dictionary from definition: ${getErrorMessage(e)}`);
  }
}

async function createCsvDictionary(
  bearerToken: string,
  dictName: string,
  fileName: string,
  fileContent: string
): Promise<void> {
  // Parse CSV to auto-detect columns
  const lines = fileContent.trim().split('\n');
  const headers = lines[0].split(',').map(h => h.trim());
  
  console.log(`CSV dictionary columns: ${headers.join(', ')}`);
  
  // Build output columns based on CSV headers
  const outputColumns = headers.map(header => ({
    name: header,
    datatype: {
      type: "string",
    },
  }));
  
  const dictUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  
  // Use first column as primary key by default
  const dictPayload = {
    name: dictName,
    settings: {
      filename: fileName,
      layout: "complex_key_hashed",
      lifetime_seconds: 300,
      format: "CSVWithNames",
      output_columns: outputColumns,
      primary_key: [headers[0]],
    },
  };
  
  console.log(`Creating CSV dictionary definition...`);
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const dictResponse = await fetch(dictUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify(dictPayload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!dictResponse.ok) {
      const errorText = await dictResponse.text();
      throw new Error(`Failed to create CSV dictionary: ${errorText}`);
    }
    
    console.log(`✓ Successfully created CSV dictionary ${dictName}`);
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create CSV dictionary: ${getErrorMessage(e)}`);
  }
}

async function createRegexpDictionary(
  bearerToken: string,
  dictName: string,
  fileName: string,
  fileContent: string
): Promise<void> {
  // Parse YAML to extract attribute columns
  const lines = fileContent.trim().split('\n');
  const attributes = new Set<string>();
  
  // Extract all unique keys from YAML
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('-') || trimmed.startsWith('#')) continue;
    if (trimmed.includes(':')) {
      const colonIndex = trimmed.indexOf(':');
      const key = trimmed.substring(0, colonIndex).trim();
      if (key && !key.startsWith('#')) {
        attributes.add(key);
      }
    }
  }
  
  // Remove 'regexp' - it's the primary key
  attributes.delete('regexp');
  
  console.log(`Regexp dictionary attributes: regexp (primary key), ${Array.from(attributes).join(', ')}`);
  
  // Build output columns: regexp + all attributes
  const outputColumns = [
    {
      name: "regexp",
      datatype: {
        type: "string",
        denullify: true,
      },
    },
    ...Array.from(attributes).map(attr => ({
      name: attr,
      datatype: {
        type: "string",
        denullify: true,
      },
    })),
  ];
  
  const dictUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
  
  const dictPayload = {
    name: dictName,
    settings: {
      filename: fileName,
      layout: "regexp_tree",
      lifetime_seconds: 5,
      format: "Regexp",
      output_columns: outputColumns,
      primary_key: ["regexp"],
      dictionary_load_level: ["ALL"],
    },
  };
  
  console.log(`Creating Regexp dictionary definition...`);
  
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);
  
  try {
    const dictResponse = await fetch(dictUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${bearerToken}`,
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
      body: JSON.stringify(dictPayload),
      signal: controller.signal,
    });
    
    clearTimeout(timeoutId);
    
    if (!dictResponse.ok) {
      const errorText = await dictResponse.text();
      throw new Error(`Failed to create regexp dictionary: ${errorText}`);
    }
    
    console.log(`✓ Successfully created Regexp dictionary ${dictName}`);
  } catch (e) {
    clearTimeout(timeoutId);
    throw new Error(`Failed to create regexp dictionary: ${getErrorMessage(e)}`);
  }
}

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
  
  // Exponential backoff configuration
  const maxRetries = 5;
  const baseDelay = 1000; // 1 second
  const maxDelay = 30000; // 30 seconds
  
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
      
      // Retry on server errors
      if (response.status >= 500 && attempt <= maxRetries) {
        const delay = calculateBackoff(attempt, baseDelay, maxDelay);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      
      // Client errors or max retries exceeded
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
      
      // Exponential backoff before retry
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

export async function insertIntoTable(
  bearerToken: string,
  fullTableName: string,
  transformName: string,
  sampleData: unknown
): Promise<void> {
  // Handle both object and array formats
  const payload = Array.isArray(sampleData) ? sampleData : [sampleData];
  
  const url = `https://${BUNDLE_TESTING_CLUSTER}/ingest/event`;
  
  const maxRetries = 20;
  const baseDelayMs = 1000; // 1 second
  const maxDelayMs = 60000; // 60 seconds
  const backoffFactor = 2.0;
  
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    // Wait with exponential backoff (except first attempt)
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
      
      // Check if this is a retryable error
      const isRetryable = 
        (status >= 500 && status <= 599) || // Server errors
        status === 408 || // Request Timeout
        status === 429; // Too Many Requests
      
      console.error(
        `Hydrolix insert failed on attempt ${attempt + 1}/${maxRetries}, ` +
        `status: ${status} (retryable: ${isRetryable}) url=${url}`
      );
      console.error(`Error response body: ${errorBody}`);
      
      // Don't retry non-retryable errors (4xx except 408 and 429)
      if (!isRetryable && status >= 400 && status < 500) {
        throw new Error(
          `Non-retryable error ${status} for ${fullTableName}: ${errorBody}`
        );
      }
      
    } catch (e) {
      console.error(`Request error on attempt ${attempt + 1}: ${getErrorMessage(e)}`);
      
      // If it's our own thrown error (non-retryable), re-throw
      const errorMsg = getErrorMessage(e);
      if (errorMsg.includes('Non-retryable')) {
        throw e;
      }
      
      // Otherwise continue to retry
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