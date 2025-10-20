// Bundle data structures

export interface Bundle {
  base_url: string;
  dashboard: Dashboard;
  other_dashboards?: Dashboard[];
  alert_rules?: AlertRules;
  method: string;
  method_overrides?: MethodOverrides;
  name: string;
  source: string;
  beta: boolean;
  tables: Table[];
  summary_tables?: SummaryTable[];
  ui: Ui;
  metadata: Metadata;
  dependencies?: Dependencies;
}

export interface Dashboard {
  path: string;
  project_var: string;
  sha256?: string;
}

export interface AlertRules {
  path: string;
  sha256?: string;
}

export interface MethodOverrides {
  region?: string;
  stream_prefix?: string;
}

export interface Table {
  dashboard_var: string;
  name: string;
  transforms: Transform[];
}

export interface SummarySqlFile {
  path: string;
  sha256?: string;
}

export interface SummaryTable {
  name: string;
  parent_table_name: string;
  dashboard_var: string;
  sql: SummarySqlFile;
}

export interface Transform {
  path: string;
  sha256?: string;
  sample?: string;
}

export interface Ui {
  primary_url: string;
  method: Graphics;
  source: Graphics;
  data_category: string;
}

export interface Graphics {
  full_title: string;
  icon_url: string;
}

export interface Metadata {
  version: string;
  maintainer: string;
  description: string;
  channel_type: string;
}

export interface Dependencies {
  grafana?: GrafanaDependencies;
  hydrolix?: HydrolixDependencies;
  "data-sources"?: DataSource[];
}

export interface GrafanaDependencies {
  version?: string;
  plugins?: GrafanaPlugin[];
}

export interface GrafanaPlugin {
  name: string;
  version: string;
}

export interface HydrolixDependencies {
  cluster_version?: string;
  required_dictionaries?: Dictionary[];
  required_functions?: Function[];
}

export interface Dictionary {
  name: string;
  source: string;
}

export interface Function {
  name: string;
  definition: string;
}

export interface DataSource {
  name: string;
  type: string;
  url: string;
  access: string;
}

export interface Output {
  cluster_domain: string;
  project_name: string;
  grafana_domain: string;
  datalink: string;
  dashboard_id: string;
  tables: OutputTable[];
}

export interface OutputTable {
  table_name: string;
  transforms: OutputTransformation[];
}

export interface OutputTransformation {
  name: string;
  data_type: string;
  data_sub_type: string;
}

// Validation functions for bundle structure
export function validateBundle(bundle: Bundle): void {
  validateHttpsUrl(bundle.base_url, "base_url");
  validateHttpsUrl(bundle.ui.primary_url, "ui.primary_url");
  
  validateMethod(bundle.method);
  validateSource(bundle.source);
  validateName(bundle.name);
  validateChannelType(bundle.metadata.channel_type);
  validateDataCategory(bundle.ui.data_category);
  
  validateDashboard(bundle.dashboard);
  
  if (bundle.other_dashboards) {
    bundle.other_dashboards.forEach(dash => validateDashboard(dash));
  }
  
  if (bundle.alert_rules) {
    validateAlertRules(bundle.alert_rules);
  }
  
  bundle.tables.forEach(table => {
    validateMacroName(table.dashboard_var);
    table.transforms.forEach(transform => {
      validateUrlPath(transform.path);
      if (transform.sha256) validateSha256(transform.sha256);
      if (transform.sample) validateUrlPath(transform.sample);
    });
  });
  
  if (bundle.summary_tables) {
    bundle.summary_tables.forEach(sumTable => {
      validateMacroName(sumTable.dashboard_var);
      validateUrlPath(sumTable.sql.path);
      if (sumTable.sql.sha256) validateSha256(sumTable.sql.sha256);
    });
  }
  
  validateHttpsUrl(bundle.ui.method.icon_url, "ui.method.icon_url");
  validateHttpsUrl(bundle.ui.source.icon_url, "ui.source.icon_url");
  
  if (bundle.dependencies?.hydrolix?.required_dictionaries) {
    bundle.dependencies.hydrolix.required_dictionaries.forEach(dict => {
      // Dictionaries can be local paths OR URLs, so don't validate as strict HTTPS
      // Just check it's not empty
      if (!dict.source || dict.source.trim().length === 0) {
        throw new Error(`dictionary.${dict.name}.source cannot be empty`);
      }
    });
  }
}

function validateDashboard(dashboard: Dashboard): void {
  validateUrlPath(dashboard.path);
  if (dashboard.sha256) validateSha256(dashboard.sha256);
}

function validateAlertRules(alertRules: AlertRules): void {
  validateUrlPath(alertRules.path);
  if (alertRules.sha256) validateSha256(alertRules.sha256);
}

function validateHttpsUrl(url: string, fieldName: string): void {
  try {
    new URL(url);
  } catch {
    throw new Error(`${fieldName}: Failed to parse URL`);
  }
  
  if (!url.startsWith("https://") && !url.startsWith("file://")) {
    throw new Error(`${fieldName}: URL must start with https:// or file://`);
  }
}

function validateUrlPath(path: string): void {
  if (path.startsWith("/") || path.includes("..")) {
    throw new Error("Path cannot start with slash or contain ..");
  }
  
  const lower = path.toLowerCase();
  if (!lower.endsWith(".json") && !lower.endsWith(".tsv") && !lower.endsWith(".sql")) {
    throw new Error("Path must end in .json, .tsv, or .sql");
  }
}

function validateMacroName(name: string): void {
  if (name.length < 5) {
    throw new Error(`${name} Must be in format __VARIABLE_NAME__`);
  }
  
  if (!name.startsWith("__") || !name.endsWith("__")) {
    throw new Error(`${name} Must be in format __VARIABLE_NAME__`);
  }
  
  const inner = name.slice(2, -2);
  
  if (inner.length === 0) {
    throw new Error(`${name} Empty inside __VARIABLE_NAME__`);
  }
  
  if (!/^[A-Z0-9_]+$/.test(inner)) {
    throw new Error(`${name} Must be upper-case or a numeral __VARIABLE_NAME__`);
  }
  
  if (inner.includes("__")) {
    throw new Error(`${name} Inside there are no double underscores`);
  }
}

function validateMethod(method: string): void {
  const validMethods = ["firehose", "s3", "kinesis", "lambda", "http_streaming", "http"];
  if (!validMethods.includes(method)) {
    throw new Error(`${method} is an invalid method`);
  }
}

function validateSource(source: string): void {
  if (!/^[a-zA-Z0-9_-]+$/.test(source)) {
    throw new Error(
      `${source} contains invalid characters (only alphanumeric, dashes, and underscores allowed)`
    );
  }
}

function validateName(name: string): void {
  if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
    throw new Error(
      `${name} contains invalid characters (only alphanumeric, dashes, and underscores allowed)`
    );
  }
}

function validateChannelType(channelType: string): void {
  const validTypes = ["AWS", "Azure", "GCP", "3rdParty", "Internal"];
  if (!validTypes.includes(channelType)) {
    throw new Error(`${channelType} is an invalid channel_type`);
  }
}

function validateDataCategory(category: string): void {
  const validCategories = ["video", "cdn", "security"];
  if (!validCategories.includes(category)) {
    throw new Error(`${category} is an invalid data category`);
  }
}

function validateSha256(hash: string): void {
  if (hash.length !== 64) {
    throw new Error(`${hash} needs to be 64 characters long`);
  }
  if (!/^[0-9a-fA-F]+$/.test(hash)) {
    throw new Error(`${hash} must contain only hexadecimal characters`);
  }
}