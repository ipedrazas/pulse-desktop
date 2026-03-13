use crate::models::ConnectorConfig;
use std::collections::HashMap;

#[derive(serde::Serialize)]
pub struct ResolvedConnector {
    pub id: String,
    pub title: String,
    pub resolved_command: Option<String>,
    pub resolved_url: Option<String>,
}

fn resolve_template(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in variables {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

#[tauri::command]
pub async fn resolve_connectors(
    connectors: Vec<ConnectorConfig>,
) -> Result<Vec<ResolvedConnector>, String> {
    let mut resolved = Vec::new();

    for conn in connectors {
        let vars: HashMap<String, String> = conn
            .variables
            .as_ref()
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let resolved_command = conn.command.as_ref().map(|c| resolve_template(c, &vars));
        let resolved_url = conn.url.as_ref().map(|u| resolve_template(u, &vars));

        resolved.push(ResolvedConnector {
            id: conn.id,
            title: conn.title,
            resolved_command,
            resolved_url,
        });
    }

    Ok(resolved)
}
