export interface Value {
    [key: string]: any;
}

export interface Transform {
    name: string;
    data_type: string;
    data_sub_type: string;
}

export interface Table {
    table_name: string;
    transforms: Transform[];
}

export interface Config {
    bundle_url: string;
    name: string;
    project_name: string;
    cluster_domain: string;
    grafana_domain: string;
    datalink: string;
    dashboard_id: string;
    tables: Table[];
}

export async function loadConfig(configPath: string): Promise<Config> {
    try {
        console.log(`Loading configuration from: ${configPath}`);
        
        // Read and parse the config file
        const configText = await Deno.readTextFile(configPath);
        const config: Config = JSON.parse(configText);

        console.log(`Loaded bundle: ${config.name}`);
        console.log(`Project: ${config.project_name}`);
        console.log(`Cluster: ${config.cluster_domain}`);
        console.log(`Tables to process: ${config.tables.length}`);

        return config;
    } catch (error) {
        throw new Error(`Failed to load config from ${configPath}: ${error.message}`);
    }
}

export function validateConfig(config: Config): void {
    const requiredFields = ['name', 'project_name', 'cluster_domain', 'tables'];
    
    for (const field of requiredFields) {
        if (!config[field]) {
            throw new Error(`Missing required field in config: ${field}`);
        }
    }

    if (!Array.isArray(config.tables) || config.tables.length === 0) {
        throw new Error('Config must contain at least one table');
    }

    for (const table of config.tables) {
        if (!table.table_name || !Array.isArray(table.transforms)) {
            throw new Error(`Invalid table configuration: ${JSON.stringify(table)}`);
        }
    }
}
