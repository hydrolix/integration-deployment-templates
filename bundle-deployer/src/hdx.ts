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
// ZIP EXTRACTION
// ============================================================================

export async function ensureZipExtracted(
  baseDir: string,
  zipFileName: string,
  targetFolder: string
): Promise<void> {
  const zipPath = `${baseDir}/${targetFolder}/${zipFileName}`;
  const extractDir = `${baseDir}/${targetFolder}/.extracted`;
  
  // Check if zip exists
  try {
    await Deno.stat(zipPath);
  } catch {
    // No zip file - that's okay, might have local files
    return;
  }
  
  // Check if already extracted
  try {
    await Deno.stat(extractDir);
    console.log(`  ✓ ${zipFileName} already extracted`);
    return;
  } catch {
    // Need to extract
  }
  
  console.log(`  Extracting ${zipFileName}...`);
  
  try {
    // Create extraction directory
    await Deno.mkdir(extractDir, { recursive: true });
    
    // Use -j flag to flatten directory structure (strip paths)
    const process = new Deno.Command("unzip", {
      args: ["-j", "-q", "-o", zipPath, "-d", extractDir],
    });
    
    const { success, stderr } = await process.output();
    
    if (!success) {
      const errorText = new TextDecoder().decode(stderr);
      throw new Error(`Unzip failed: ${errorText}`);
    }
    
    console.log(`  ✓ Extracted ${zipFileName} to .extracted/`);
  } catch (e) {
    throw new Error(`Failed to extract ${zipFileName}: ${getErrorMessage(e)}`);
  }
}

export async function discoverDictionaries(baseDir: string): Promise<string[]> {
  const discovered: string[] = [];
  
  // Check .extracted/ (flattened) and root dictionaries/
  const possibleDirs = [
    `${baseDir}/dictionaries/.extracted`,
    `${baseDir}/dictionaries`
  ];
  
  for (const dir of possibleDirs) {
    try {
      await Deno.stat(dir);
      console.log(`  Scanning for dictionaries in ${dir.split('/').slice(-2).join('/')}...`);
      
      for await (const entry of Deno.readDir(dir)) {
        if (entry.isFile && entry.name.endsWith('.json')) {
          const baseName = entry.name.replace('.json', '');
          
          // Skip if already found (avoid duplicates)
          if (discovered.includes(baseName)) {
            continue;
          }
          
          // Check if matching data file exists
          const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
          for (const ext of possibleExtensions) {
            try {
              await Deno.stat(`${dir}/${baseName}.${ext}`);
              discovered.push(baseName);
              console.log(`    Found: ${baseName} (.json + .${ext})`);
              break;
            } catch {
              // Try next extension
            }
          }
        }
      }
    } catch {
      // Directory doesn't exist, try next
    }
  }
  
  return discovered;
}

export async function discoverFunctions(baseDir: string): Promise<string[]> {
  const discovered: string[] = [];
  const functionsDir = `${baseDir}/functions`;
  
  try {
    await Deno.stat(functionsDir);
  } catch {
    return [];
  }
  
  console.log("  Scanning for functions in functions/...");
  
  for await (const entry of Deno.readDir(functionsDir)) {
    if (entry.isFile && entry.name.endsWith('.json')) {
      const functionName = entry.name.replace('.json', '');
      discovered.push(functionName);
      console.log(`    Found: ${functionName}`);
    }
  }
  
  return discovered;
}

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
  
  // Look for function file
  const functionFilePath = `${baseDir}/functions/${functionName}.json`;
  
  try {
    await Deno.stat(functionFilePath);
  } catch {
    throw new Error(
      `Bundle-specific function '${functionName}' declared but file not found.\n` +
      `  Expected: ${functionFilePath}\n` +
      `  Actions:\n` +
      `    1. Add ${functionName}.json to functions/ folder, OR\n` +
      `    2. Remove '${functionName}' from required_functions in bundle.json if not needed`
    );
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

async function findDictionaryFiles(
  baseDir: string,
  dictionaryName: string
): Promise<{ jsonPath: string; dataPath: string } | null> {
  // Try root first (for overrides), then .extracted/ (flattened)
  const searchPaths = [
    `${baseDir}/dictionaries`,
    `${baseDir}/dictionaries/.extracted`
  ];
  
  for (const dir of searchPaths) {
    const jsonPath = `${dir}/${dictionaryName}.json`;
    
    try {
      await Deno.stat(jsonPath);
      
      // Found JSON, now find data file
      const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
      for (const ext of possibleExtensions) {
        const dataPath = `${dir}/${dictionaryName}.${ext}`;
        try {
          await Deno.stat(dataPath);
          return { jsonPath, dataPath };
        } catch {
          // Try next extension
        }
      }
      
      // Found JSON but no data file
      throw new Error(`Found ${jsonPath} but no matching data file`);
    } catch (e) {
      if (e instanceof Error && e.message.includes('no matching data file')) {
        throw e;
      }
      // JSON not found in this directory, try next
    }
  }
  
  return null;
}

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
  
  // Find dictionary files (checks .extracted/ then root)
  const files = await findDictionaryFiles(baseDir, dictionaryName);
  
  if (!files) {
  throw new Error(
      `Shared dictionary '${dictionaryName}' declared but files not found.\n` +
      `  Expected:\n` +
      `    - ${baseDir}/dictionaries/${dictionaryName}.json (definition)\n` +
      `    - ${baseDir}/dictionaries/${dictionaryName}.[csv/yaml/yml/tsv] (data)\n` +
      `  Actions:\n` +
      `    1. Add ${dictionaryName}.json + data file to dictionaries/ folder, OR\n` +
      `    2. Check if files exist in dictionaries.zip, OR\n` +
      `    3. Remove '${dictionaryName}' from shared_dictionaries in bundle.json if not needed`
    );
  }
  
  console.log(`  Found files: ${files.jsonPath} + ${files.dataPath}`);
  
  // Read dictionary definition
  let dictDef;
  try {
    const content = await Deno.readTextFile(files.jsonPath);
    dictDef = JSON.parse(content);
  } catch (e) {
    throw new Error(`Failed to read dictionary definition ${files.jsonPath}: ${getErrorMessage(e)}`);
  }
  
  // Read data file
  const dataFileContent = await Deno.readTextFile(files.dataPath);
  const fileName = files.dataPath.split('/').pop()!;
  
  // Upload and create
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
  
  // Strip extension for the name (Hydrolix references without extension)
  const baseFileName = fileName.replace(/\.(csv|yaml|yml|tsv)$/i, '');
  
  try {
    const filesListResponse = await fetch(filesUrl, {
      headers: { 'Authorization': `Bearer ${bearerToken}` },
    });
    
    if (filesListResponse.ok) {
      const existingFiles = await filesListResponse.json();
      
      if (Array.isArray(existingFiles)) {
        // Check for file with or without extension
        const fileExists = existingFiles.some((f: any) => {
          const name = typeof f === 'string' ? f : f.name;
          return name === baseFileName || name === fileName;
        });
        
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
  formData.append('name', baseFileName);  // ← Upload WITHOUT extension
  
  try {
    console.log(`  Uploading dictionary file: ${fileName} (as ${baseFileName})...`);
    
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
    
    console.log(`  ✓ Uploaded dictionary file: ${baseFileName}`);
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
      
      // Get error details
      const errorBody = await response.text();
      console.error(`\n❌ Transform validation failed (attempt ${attempt}/${maxRetries}):`);
      console.error(`   Status: ${response.status}`);
      console.error(`   Error: ${errorBody}`);
      
      if (response.status >= 500 && attempt <= maxRetries) {
        const delay = calculateBackoff(attempt, baseDelay, maxDelay);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      
      // On 400 errors, don't retry - it won't help
      throw new Error(
        `Hydrolix add transform failed, status: ${response.status}, error: ${errorBody}`
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