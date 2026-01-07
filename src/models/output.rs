use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct OutputTransformation {
    pub name: String,
    pub data_type: String,
    pub data_sub_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct OutputTable {
    pub table_name: String,
    pub transforms: Vec<OutputTransformation>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub bundle_url: String,
    pub name: String,
    pub project_name: String,
    pub cluster_domain: String,
    pub grafana_domain: String,
    pub datalink: String,
    pub dashboard_id: String,
    pub tables: Vec<OutputTable>,
}
