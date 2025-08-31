/*

# Generate 100 records (default)
deno run --allow-net --allow-env --allow-read generate-zuplo-fake-data.ts test-config.json

# Generate 500 records
deno run --allow-net --allow-env --allow-read generate-zuplo-fake-data.ts test-config.json 500

# Generate records continuously until stopped
deno run --allow-net --allow-env --allow-read generate-zuplo-fake-data.ts test-config.json forever

*/


import { getAllGeoData, getRandomGeoData, type GeoData }
    from "https://raw.githubusercontent.com/hydrolix/integration-deployment-templates/refs/heads/main/automation/traffic-generation/geo-data.ts";

import { randomUserAgent, randomHttpMethod, randomHttpStatusCode, randomOrganizationName, randomRoutePath }
    from "https://raw.githubusercontent.com/hydrolix/integration-deployment-templates/refs/heads/main/automation/traffic-generation/http-things.ts";

import { loadConfig, validateConfig, type Config, type Value }
    from "https://raw.githubusercontent.com/hydrolix/integration-deployment-templates/refs/heads/main/automation/traffic-generation/config-reader.ts";

// main.ts
const HTTP_TIMEOUT = 120;
const AWS_ACCOUNT_ID = "123456789012";

function json(obj: any): Value {
    return obj;
}

async function processRequest(config: Config, recordCount: number | 'forever'): Promise<void> {
    try {
        // Get auth token
        const bearerToken = await getAuthToken(config.cluster_domain);

        // Process each table and its transforms
        for (const table of config.tables) {
            for (const transform of table.transforms) {
                await processTransform(bearerToken, config.project_name + "." + table.table_name, transform.name, config, recordCount);
            }
        }

    } catch (error) {
        throw new Error(`ERROR: processRequest failed: ${error.message}`);
    }
}

async function processTransform(
    bearerToken: string,
    tableName: string,
    transformName: string,
    config: Config,
    recordCount: number | 'forever'
): Promise<void> {
    console.log(`Processing transform ${transformName} for table ${tableName}`);

    if (recordCount === 'forever') {
        console.log("Running in forever mode - will generate records until stopped");
        let counter = 0;
        while (true) {
            const payload = fakeDataPayload();
            console.log(`Record ${++counter}: ${JSON.stringify(payload).substring(0, 100)}...`);
            await sendIt(bearerToken, tableName, transformName, payload, config.cluster_domain);

            // Add a small delay between records in forever mode to avoid overwhelming the system
            await new Promise(resolve => setTimeout(resolve, 100));
        }
    } else {
        console.log(`Generating ${recordCount} records`);
        for (let j = 0; j < recordCount; j++) {
            const payload = fakeDataPayload();
            console.log(`Record ${j + 1}/${recordCount}: ${JSON.stringify(payload).substring(0, 100)}...`);
            await sendIt(bearerToken, tableName, transformName, payload, config.cluster_domain);
        }
    }
    console.log(`Done with: ${transformName}`);
}

async function sendIt(
    bearerToken: string,
    projectTable: string,
    transformName: string,
    payload: Value,
    clusterDomain: string
): Promise<void> {
    await insertIntoTable(bearerToken, projectTable, transformName, payload, clusterDomain);
}

function fakeDataPayload(): Value {
    let geoData = getRandomGeoData();

    return json({
        "accountName": randomAccountName(),
        "asOrganization": randomOrganizationName(),
        "asn": randomAsn(),
        "city": geoData.city,
        "clientIP": geoData.clientIP,
        "colo": geoData.colo,
        "continent": geoData.continent,
        "country": geoData.country,
        "deploymentName": `metrics-egress-${randomDeploymentId()}`,
        "durationMs": Math.floor(Math.random() * (2000 - 100) + 100),
        "instanceId": crypto.randomUUID(),
        "latitude": geoData.latitude,
        "logSourceVersion": "1.0",
        "longitude": geoData.longitude,
        "method": randomHttpMethod(),
        "metroCode": geoData.metroCode,
        "operationId": "metrics-egress",
        "postalCode": geoData.postalCode,
        "projectName": "EgressMetrics",
        "region": geoData.region,
        "regionCode": geoData.regionCode,
        "requestId": crypto.randomUUID(),
        "routePath": randomRoutePath(),
        "statusCode": randomHttpStatusCode(),
        "systemRouteName": "EgressBytes",
        "timestamp": zuplo_timestamp(),
        "timezone": geoData.timezone,
        "url": `https://api.metrics.io/${randomApiId()}`,
        "userSub": AWS_ACCOUNT_ID,
        "zuploUserAgent": randomUserAgent(),
    });
}

// Helper functions for generating random data
function randomAccountName(): string {
    const accounts = ["production account", "staging account", "test account", "development account", "qa account"];
    return accounts[Math.floor(Math.random() * accounts.length)];
}

function randomAsn(): string {
    const asns = ["150814", "16509", "15169", "8075", "14061", "13335"];
    return asns[Math.floor(Math.random() * asns.length)];
}

function randomDeploymentId(): string {
    return Math.random().toString(36).substring(2, 15);
}

function randomApiId(): string {
    return Math.random().toString(16).substring(2, 34);
}

async function getAuthToken(clusterDomain: string): Promise<string> {
    const FAKE_TRAFFIC_USERNAME = Deno.env.get("FAKE_TRAFFIC_USERNAME") || "";
    const FAKE_TRAFFIC_PASSWORD = Deno.env.get("FAKE_TRAFFIC_PASSWORD") || "";

    const response = await fetch(`https://${clusterDomain}/config/v1/login`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            username: FAKE_TRAFFIC_USERNAME,
            password: FAKE_TRAFFIC_PASSWORD
        })
    });

    if (!response.ok) {
        throw new Error(`Login failed: ${response.statusText}`);
    }

    const jsonData = await response.json();
    const token = jsonData.auth_token?.access_token;

    if (!token) {
        throw new Error("Could not find token in payload");
    }

    return token;
}

async function insertIntoTable(
    bearerToken: string,
    fullTableName: string,
    transformName: string,
    sampleData: Value,
    clusterDomain: string
): Promise<void> {
    const sampleDataArray = json([sampleData]);
    const url = `https://${clusterDomain}/ingest/event`;

    const maxRetries = 20;
    const baseDelayMs = 1000;
    const maxDelayMs = 60000;
    const backoffFactor = 2;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
        if (attempt > 0) {
            const exponentialDelay = baseDelayMs * Math.pow(backoffFactor, attempt - 1);
            const finalDelay = Math.min(exponentialDelay, maxDelayMs);

            console.log(`Retry attempt ${attempt + 1}/${maxRetries}, waiting ${finalDelay}ms`);
            await new Promise(resolve => setTimeout(resolve, finalDelay));
        }

        let response: Response;
        try {
            response = await fetch(url, {
                method: "POST",
                headers: {
                    "Authorization": `Bearer ${bearerToken}`,
                    "Content-Type": "application/json",
                    "Accept": "application/json",
                    "x-hdx-table": fullTableName,
                    "x-hdx-transform": transformName
                },
                body: JSON.stringify(sampleDataArray)
            });
        } catch (error) {
            console.error(`Request error on attempt ${attempt + 1}: ${error.message}`);
            continue;
        }

        if (response.ok) {
            if (attempt > 0) {
                console.log(`Successfully inserted data after ${attempt + 1} retries`);
            }
            return;
        }

        const status = response.status;
        const errorBody = await response.text().catch(() => "");

        const isRetryable = (status >= 500 && status <= 599) || status === 408 || status === 429;
        const isClientError = status >= 400 && status <= 499 && status !== 408 && status !== 429;

        console.error(
            `Hydrolix insert failed on attempt ${attempt + 1}/${maxRetries}, status: ${status} (retryable: ${isRetryable})`
        );
        console.error(`Error response body: ${errorBody}`);

        if (isClientError) {
            throw new Error(`Non-retryable error ${status} for ${fullTableName}: ${errorBody}`);
        }

        if (attempt === maxRetries - 1) {
            throw new Error(`Failed to send data to ${fullTableName} after ${maxRetries} attempts`);
        }
    }
}

// Main execution
if (import.meta.main) {
    try {
        // Check if arguments were provided
        if (Deno.args.length < 1) {
            console.error("Usage: deno run main.ts <config-file.json> [record-count|forever]");
            console.error("Examples:");
            console.error("  deno run main.ts test-config.json 100     # Generate 100 records");
            console.error("  deno run main.ts test-config.json forever # Generate records continuously");
            Deno.exit(1);
        }

        const configPath = Deno.args[0];
        let recordCount: number | 'forever' = 100; // Default to 100 records

        // Parse the record count argument if provided
        if (Deno.args.length >= 2) {
            const countArg = Deno.args[1].toLowerCase();
            if (countArg === 'forever') {
                recordCount = 'forever';
            } else {
                const parsedCount = parseInt(countArg, 10);
                if (isNaN(parsedCount) || parsedCount <= 0) {
                    console.error("Record count must be a positive number or 'forever'");
                    Deno.exit(1);
                }
                recordCount = parsedCount;
            }
        }

        console.log(`Loading configuration from: ${configPath}`);
        console.log(`Record generation mode: ${recordCount === 'forever' ? 'continuous' : `${recordCount} records`}`);

        const config = await loadConfig(configPath);
        validateConfig(config);

        console.log(`Processing bundle: ${config.name}`);
        console.log(`Project: ${config.project_name}`);
        console.log(`Cluster: ${config.cluster_domain}`);
        console.log(`Tables to process: ${config.tables.length}`);

        await processRequest(config, recordCount);

        if (recordCount !== 'forever') {
            console.log("Process completed successfully");
        }
    } catch (error) {
        console.error("Process failed:", error.message);
        Deno.exit(1);
    }
}

function zuplo_timestamp() {
    let date = new Date();

    const year = date.getUTCFullYear();
    const month = String(date.getUTCMonth() + 1).padStart(2, '0');
    const day = String(date.getUTCDate()).padStart(2, '0');
    const hours = String(date.getUTCHours()).padStart(2, '0');
    const minutes = String(date.getUTCMinutes()).padStart(2, '0');
    const seconds = String(date.getUTCSeconds()).padStart(2, '0');
    const milliseconds = String(date.getUTCMilliseconds()).padStart(3, '0');

    return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.${milliseconds}Z`;
}

