use serde_json::Value;

pub fn rewrite_datasource_uid(dashboard: &mut Value, target_uid: &str) {
    walk(dashboard, target_uid);
}

fn walk(value: &mut Value, target_uid: &str) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(ds)) = map.get_mut("datasource") {
                if let Some(uid) = ds.get_mut("uid") {
                    *uid = Value::String(target_uid.to_string());
                }
            }
            for (_, v) in map.iter_mut() {
                walk(v, target_uid);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                walk(item, target_uid);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn load(name: &str) -> Value {
        let raw = match name {
            "single_panel" => include_str!("fixtures/dashboard_rewrite/single_panel.json"),
            "template_variables" => {
                include_str!("fixtures/dashboard_rewrite/template_variables.json")
            }
            "with_inputs" => include_str!("fixtures/dashboard_rewrite/with_inputs.json"),
            "with_elements" => include_str!("fixtures/dashboard_rewrite/with_elements.json"),
            "nested_rows" => include_str!("fixtures/dashboard_rewrite/nested_rows.json"),
            "mixed_types" => include_str!("fixtures/dashboard_rewrite/mixed_types.json"),
            _ => panic!("unknown fixture: {name}"),
        };
        serde_json::from_str(raw).expect("fixture is valid JSON")
    }

    #[test]
    fn rewrites_single_panel_datasource_uid() {
        let mut dashboard = load("single_panel");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        assert_eq!(
            dashboard["panels"][0]["datasource"]["uid"]
                .as_str()
                .unwrap(),
            "aelocpydfc9vke"
        );
    }

    #[test]
    fn rewrites_template_variable_datasource_uid() {
        let mut dashboard = load("template_variables");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        assert_eq!(
            dashboard["templating"]["list"][0]["datasource"]["uid"]
                .as_str()
                .unwrap(),
            "aelocpydfc9vke"
        );
    }

    #[test]
    fn rewrites_inputs_block_datasource_uid() {
        let mut dashboard = load("with_inputs");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        assert_eq!(
            dashboard["__inputs"][0]["datasource"]["uid"]
                .as_str()
                .unwrap(),
            "aelocpydfc9vke"
        );
    }

    #[test]
    fn rewrite_is_idempotent_when_already_target() {
        let mut dashboard = load("mixed_types");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        let after_first = dashboard.clone();
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        assert_eq!(dashboard, after_first);
    }

    #[test]
    fn rewrites_all_uids_preserves_types_for_mixed_datasources() {
        let mut dashboard = load("mixed_types");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        let panels = dashboard["panels"].as_array().unwrap();
        for p in panels {
            assert_eq!(p["datasource"]["uid"].as_str().unwrap(), "aelocpydfc9vke");
        }
        assert_eq!(
            panels[0]["datasource"]["type"].as_str().unwrap(),
            "hydrolix"
        );
        assert_eq!(panels[1]["datasource"]["type"].as_str().unwrap(), "grafana");
        assert_eq!(
            panels[2]["datasource"]["type"].as_str().unwrap(),
            "prometheus"
        );
    }

    #[test]
    fn rewrites_panels_nested_in_row() {
        let mut dashboard = load("nested_rows");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        let row_panels = &dashboard["panels"][0]["panels"];
        assert_eq!(
            row_panels[0]["datasource"]["uid"].as_str().unwrap(),
            "aelocpydfc9vke"
        );
        assert_eq!(
            row_panels[1]["datasource"]["uid"].as_str().unwrap(),
            "aelocpydfc9vke"
        );
        assert_eq!(
            dashboard["panels"][1]["datasource"]["uid"]
                .as_str()
                .unwrap(),
            "aelocpydfc9vke"
        );
    }

    #[test]
    fn rewrites_elements_block_datasource_uid() {
        let mut dashboard = load("with_elements");
        rewrite_datasource_uid(&mut dashboard, "aelocpydfc9vke");
        let model = &dashboard["__elements"]["lib_abc"]["model"];
        assert_eq!(
            model["datasource"]["uid"].as_str().unwrap(),
            "aelocpydfc9vke"
        );
        assert_eq!(
            model["targets"][0]["datasource"]["uid"].as_str().unwrap(),
            "aelocpydfc9vke"
        );
    }
}
