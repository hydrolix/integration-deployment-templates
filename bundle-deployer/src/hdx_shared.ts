// Shared resource management for hdx_solutions project
// NEW FILE - Handles functions and dictionaries that are shared across all bundles

import { getErrorMessage } from "./utils/error.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const SHARED_PROJECT_NAME = Deno.env.get("SHARED_PROJECT_NAME") || "hdx_solutions";
const IS_LOCAL = Deno.args.includes("--local") || Deno.args.includes("--local-dashboard-only");
const HTTP_TIMEOUT = 120000;

let SHARED_PROJECT_UUID: string | null = null;

// ============================================================================
// SHARED PROJECT MANAGEMENT
// ============================================================================

export async function ensureSharedProjectExists(bearerToken: string): Promise<string> {
    if (SHARED_PROJECT_UUID) {
        return SHARED_PROJECT_UUID;
    }

    console.log(`Checking for shared project: ${SHARED_PROJECT_NAME}...`);

    const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/`;

    try {
        const response = await fetch(listUrl, {
            headers: { 'Authorization': `Bearer ${bearerToken}` },
        });

        if (!response.ok) {
            throw new Error(`Failed to list projects: ${response.statusText}`);
        }

        const projects = await response.json();
        const existing = Array.isArray(projects) ? projects : projects?.results || projects?.projects || projects?.data || [];

        const sharedProject = existing.find((p: any) => p.name === SHARED_PROJECT_NAME);

        if (sharedProject) {
            SHARED_PROJECT_UUID = sharedProject.uuid;
            console.log(`  ✓ Shared project exists (uuid: ${SHARED_PROJECT_UUID})`);
            return SHARED_PROJECT_UUID;
        }

        // Project doesn't exist - create it
        console.log(`  Creating shared project: ${SHARED_PROJECT_NAME}...`);
        SHARED_PROJECT_UUID = await createSharedProject(bearerToken);
        console.log(`  ✓ Created shared project (uuid: ${SHARED_PROJECT_UUID})`);

        return SHARED_PROJECT_UUID;
    } catch (e) {
        throw new Error(`Failed to ensure shared project exists: ${getErrorMessage(e)}`);
    }
}

async function createSharedProject(bearerToken: string): Promise<string> {
    const createUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/`;

    const payload = {
        name: SHARED_PROJECT_NAME,
        description: "Shared resources for all bundles (functions, dictionaries)",
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);

    try {
        const response = await fetch(createUrl, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${bearerToken}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload),
            signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`HTTP ${response.status}: ${errorText}`);
        }

        const result = await response.json();
        return result.uuid;
    } catch (e) {
        clearTimeout(timeoutId);
        throw new Error(`Failed to create shared project: ${getErrorMessage(e)}`);
    }
}

// ============================================================================
// SHARED FUNCTIONS
// ============================================================================

export async function checkAndCreateSharedFunction(
    bearerToken: string,
    functionName: string,
    baseDir: string
): Promise<void> {
    console.log(`Checking shared function: ${functionName}...`);

    await ensureSharedProjectExists(bearerToken);

    const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${SHARED_PROJECT_UUID}/functions/`;
    const expectedName = `${SHARED_PROJECT_NAME}_${functionName}`;

    const listResponse = await fetch(listUrl, {
        headers: { 'Authorization': `Bearer ${bearerToken}` },
    });

    if (listResponse.ok) {
        const responseData = await listResponse.json();
        let existing: Array<{ name: string }> = [];

        if (Array.isArray(responseData)) {
            existing = responseData;
        } else if (responseData?.results) {
            existing = responseData.results;
        } else if (responseData?.functions) {
            existing = responseData.functions;
        } else if (responseData?.data) {
            existing = responseData.data;
        }

        if (existing.some(f => f.name === functionName)) {
            console.log(`  ✓ Shared function ${functionName} exists (as ${SHARED_PROJECT_NAME}_${functionName})`);
            return;
        }
    } else {
        throw new Error(`Failed to list functions: ${listResponse.status} ${listResponse.statusText}`);
    }

    // Create the function
    const functionFilePath = `${baseDir}/functions/${functionName}.json`;

    try {
        await Deno.stat(functionFilePath);
    } catch {
        throw new Error(
            `Shared function '${functionName}' declared but file not found.\n` +
            `  Expected: ${functionFilePath}\n` +
            `  Actions:\n` +
            `    1. Add ${functionName}.json to functions/ folder, OR\n` +
            `    2. Remove '${functionName}' from shared_functions in bundle.json if not needed`
        );
    }

    let functionDef;
    try {
        const content = await Deno.readTextFile(functionFilePath);
        functionDef = JSON.parse(content);
    } catch (e) {
        throw new Error(`Failed to read shared function file: ${getErrorMessage(e)}`);
    }

    // Replace template variables
    if (functionDef.sql && typeof functionDef.sql === 'string') {
        functionDef.sql = functionDef.sql
            .replace(/__SHARED_PROJECT__/g, SHARED_PROJECT_NAME)
            .replace(/__PROJECT_NAME__/g, SHARED_PROJECT_NAME); // Fallback for old templates
    }

    const createUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${SHARED_PROJECT_UUID}/functions/`;

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);

    try {
        console.log(`  Creating shared function ${functionName} (will become ${expectedName})...`);

        const response = await fetch(createUrl, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${bearerToken}`,
                'Content-Type': 'application/json',
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

        console.log(`  ✓ Created shared function ${functionName}`);
    } catch (e) {
        clearTimeout(timeoutId);
        throw new Error(`Failed to create shared function: ${getErrorMessage(e)}`);
    }
}

// ============================================================================
// SHARED DICTIONARIES
// ============================================================================

export async function checkAndCreateSharedDictionary(
    bearerToken: string,
    dictionaryName: string,
    baseDir: string
): Promise<void> {
    console.log(`Checking shared dictionary: ${dictionaryName}...`);

    await ensureSharedProjectExists(bearerToken);

    const listUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${SHARED_PROJECT_UUID}/dictionaries/`;
    const expectedName = `${SHARED_PROJECT_NAME}_${dictionaryName}`;

    try {
        const listResponse = await fetch(listUrl, {
            headers: { 'Authorization': `Bearer ${bearerToken}` },
        });

        if (listResponse.ok) {
            const responseData = await listResponse.json();
            let existing: Array<{ name: string }> = [];

            if (Array.isArray(responseData)) {
                existing = responseData;
            } else if (responseData?.results) {  // ← ADD THIS FIRST
                existing = responseData.results;
            } else if (responseData?.dictionaries) {
                existing = responseData.dictionaries;
            } else if (responseData?.data) {
                existing = responseData.data;
            }

            if (existing.some(d => d.name === dictionaryName)) {
                console.log(`  ✓ Shared dictionary ${dictionaryName} exists (as ${SHARED_PROJECT_NAME}_${dictionaryName})`);
                return;
            }
        }
    } catch (e) {
        console.warn(`  ⚠️  Could not check for existing shared dictionary: ${getErrorMessage(e)}`);
    }

    // Create the dictionary
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

    let dictDef;
    try {
        const content = await Deno.readTextFile(files.jsonPath);
        dictDef = JSON.parse(content);
    } catch (e) {
        throw new Error(`Failed to read shared dictionary definition: ${getErrorMessage(e)}`);
    }

    const dataFileContent = await Deno.readTextFile(files.dataPath);
    const fileName = files.dataPath.split('/').pop()!;

    await uploadSharedDictionaryFile(bearerToken, fileName, dataFileContent);
    await createSharedDictionaryDefinition(bearerToken, dictionaryName, dictDef);

    console.log(`  ✓ Created shared dictionary ${dictionaryName}`);
}

async function findDictionaryFiles(
    baseDir: string,
    dictionaryName: string
): Promise<{ jsonPath: string; dataPath: string } | null> {
    const searchPaths = [
        `${baseDir}/dictionaries`,
        `${baseDir}/dictionaries/.extracted`
    ];

    for (const dir of searchPaths) {
        const jsonPath = `${dir}/${dictionaryName}.json`;

        try {
            await Deno.stat(jsonPath);

            const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
            for (const ext of possibleExtensions) {
                const dataPath = `${dir}/${dictionaryName}.${ext}`;
                try {
                    await Deno.stat(dataPath);
                    return { jsonPath, dataPath };
                } catch {
                    continue;
                }
            }

            throw new Error(`Found ${jsonPath} but no matching data file`);
        } catch (e) {
            if (e instanceof Error && e.message.includes('no matching data file')) {
                throw e;
            }
            continue;
        }
    }

    return null;
}

async function uploadSharedDictionaryFile(
    bearerToken: string,
    fileName: string,
    fileContent: string
): Promise<void> {
    const filesUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${SHARED_PROJECT_UUID}/dictionaries/files/`;
    const baseFileName = fileName.replace(/\.(csv|yaml|yml|tsv)$/i, '');

    try {
        const filesListResponse = await fetch(filesUrl, {
            headers: { 'Authorization': `Bearer ${bearerToken}` },
        });

        if (filesListResponse.ok) {
            const existingFiles = await filesListResponse.json();

            if (Array.isArray(existingFiles)) {
                const fileExists = existingFiles.some((f: any) => {
                    const name = typeof f === 'string' ? f : f.name;
                    return name === baseFileName || name === fileName;
                });

                if (fileExists) {
                    console.log(`  ✓ Shared dictionary file already uploaded: ${fileName}`);
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
    formData.append('name', baseFileName);

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);

    try {
        console.log(`  Uploading shared dictionary file: ${fileName} (as ${baseFileName})...`);

        const uploadResponse = await fetch(filesUrl, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${bearerToken}`,
            },
            body: formData,
            signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!uploadResponse.ok) {
            const errorText = await uploadResponse.text();
            throw new Error(`Failed to upload: ${errorText}`);
        }

        console.log(`  ✓ Uploaded shared dictionary file: ${baseFileName}`);
    } catch (e) {
        clearTimeout(timeoutId);
        throw new Error(`Failed to upload shared dictionary file: ${getErrorMessage(e)}`);
    }
}

async function createSharedDictionaryDefinition(
    bearerToken: string,
    dictionaryName: string,
    dictDefinition: any
): Promise<void> {
    const dictUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${SHARED_PROJECT_UUID}/dictionaries/`;
    const expectedName = `${SHARED_PROJECT_NAME}_${dictionaryName}`;

    const payload = {
        ...dictDefinition,
        name: dictionaryName,
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HTTP_TIMEOUT);

    try {
        console.log(`  Creating shared dictionary definition: ${dictionaryName} (will become ${expectedName})...`);

        const dictResponse = await fetch(dictUrl, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${bearerToken}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload),
            signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!dictResponse.ok) {
            const errorText = await dictResponse.text();
            throw new Error(`HTTP ${dictResponse.status}: ${errorText}`);
        }

        console.log(`  ✓ Created shared dictionary definition`);
    } catch (e) {
        clearTimeout(timeoutId);
        throw new Error(`Failed to create shared dictionary definition: ${getErrorMessage(e)}`);
    }
}

export function getSharedProjectName(): string {
    return SHARED_PROJECT_NAME;
}