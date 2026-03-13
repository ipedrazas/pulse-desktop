use crate::models::ConnectorConfig;
use std::collections::HashMap;
use std::process::Stdio;
use tauri::{Emitter, Window};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(serde::Serialize)]
pub struct ResolvedConnector {
    pub id: String,
    pub title: String,
    pub resolved_command: Option<String>,
    pub resolved_url: Option<String>,
}

#[derive(Clone, serde::Serialize)]
struct ConnectorLogEvent {
    connector_id: String,
    line: String,
    stream: String,
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

/// Execute a connector command and stream its output to the frontend
#[tauri::command]
pub async fn stream_connector(
    window: Window,
    connector_id: String,
    command: String,
    cwd: Option<String>,
) -> Result<u32, String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut cmd = tokio::process::Command::new(parts[0]);
    cmd.args(&parts[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref dir) = cwd {
        cmd.current_dir(dir);
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let pid = child.id().unwrap_or(0);

    // Stream stdout
    let stdout = child.stdout.take();
    let window_stdout = window.clone();
    let id_stdout = connector_id.clone();
    tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = window_stdout.emit(
                    "connector:log",
                    ConnectorLogEvent {
                        connector_id: id_stdout.clone(),
                        line,
                        stream: "stdout".to_string(),
                    },
                );
            }
        }
    });

    // Stream stderr
    let stderr = child.stderr.take();
    let window_stderr = window.clone();
    let id_stderr = connector_id.clone();
    tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = window_stderr.emit(
                    "connector:log",
                    ConnectorLogEvent {
                        connector_id: id_stderr.clone(),
                        line,
                        stream: "stderr".to_string(),
                    },
                );
            }
        }
    });

    // Wait for exit in background
    let window_done = window;
    let id_done = connector_id;
    tokio::spawn(async move {
        let status = child.wait().await;
        let exit_code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
        let _ = window_done.emit(
            "connector:done",
            serde_json::json!({
                "connector_id": id_done,
                "exit_code": exit_code,
            }),
        );
    });

    Ok(pid)
}
