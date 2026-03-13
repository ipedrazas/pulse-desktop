use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use std::process::Command;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2Check {
    pub name: String,
    pub status: String, // "pass", "fail", "warning"
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fix_macro: Option<String>, // linked macro id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2Result {
    pub checks: Vec<A2Check>,
    pub pass_count: usize,
    pub fail_count: usize,
    pub warning_count: usize,
}

#[tauri::command]
pub async fn run_a2(
    pool: State<'_, SqlitePool>,
    project_id: String,
    root_path: String,
    fix_hints: Option<Vec<FixHint>>,
) -> Result<A2Result, String> {
    let root = Path::new(&root_path);

    // Check if a2 is available
    if which::which("a2").is_err() {
        return Err("a2 binary not found on PATH".to_string());
    }

    // Check for .a2.yaml
    if !root.join(".a2.yaml").exists() {
        return Err("No .a2.yaml found in project root".to_string());
    }

    let output = Command::new("a2")
        .args(["-f", "json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run a2: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    // Store the run
    let _ = sqlx::query(
        "INSERT INTO runs (project_id, kind, status, command, cwd, exit_code, finished_at)
         VALUES (?, 'a2', ?, 'a2 -f json', ?, ?, datetime('now'))",
    )
    .bind(&project_id)
    .bind(if output.status.success() { "success" } else { "failure" })
    .bind(&root_path)
    .bind(exit_code)
    .execute(pool.inner())
    .await;

    // Store raw output as log
    let run_id: Option<i64> = sqlx::query_scalar("SELECT MAX(id) FROM runs WHERE project_id = ?")
        .bind(&project_id)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

    if let Some(run_id) = run_id {
        let _ = sqlx::query(
            "INSERT INTO run_logs (run_id, stream, chunk) VALUES (?, 'stdout', ?)",
        )
        .bind(run_id)
        .bind(stdout.as_ref())
        .execute(pool.inner())
        .await;

        if !output.stderr.is_empty() {
            let _ = sqlx::query(
                "INSERT INTO run_logs (run_id, stream, chunk) VALUES (?, 'stderr', ?)",
            )
            .bind(run_id)
            .bind(String::from_utf8_lossy(&output.stderr).as_ref())
            .execute(pool.inner())
            .await;
        }
    }

    // Parse a2 JSON output
    let checks = parse_a2_output(&stdout, fix_hints.as_deref());

    let pass_count = checks.iter().filter(|c| c.status == "pass").count();
    let fail_count = checks.iter().filter(|c| c.status == "fail").count();
    let warning_count = checks.iter().filter(|c| c.status == "warning").count();

    Ok(A2Result {
        checks,
        pass_count,
        fail_count,
        warning_count,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixHint {
    pub check_pattern: String,
    pub macro_id: String,
}

fn parse_a2_output(stdout: &str, fix_hints: Option<&[FixHint]>) -> Vec<A2Check> {
    let mut checks = Vec::new();

    // Try parsing as JSON array or object
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) {
        let items = match &value {
            serde_json::Value::Array(arr) => arr.clone(),
            serde_json::Value::Object(obj) => {
                // Try "checks" or "results" key
                obj.get("checks")
                    .or_else(|| obj.get("results"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| vec![value.clone()])
            }
            _ => vec![],
        };

        for item in items {
            let name = item["name"]
                .as_str()
                .or_else(|| item["rule"].as_str())
                .or_else(|| item["id"].as_str())
                .unwrap_or("unknown")
                .to_string();

            let status = item["status"]
                .as_str()
                .or_else(|| item["result"].as_str())
                .map(|s| match s.to_lowercase().as_str() {
                    "pass" | "ok" | "success" => "pass",
                    "fail" | "error" | "failure" => "fail",
                    "warn" | "warning" => "warning",
                    _ => "fail",
                })
                .unwrap_or("fail")
                .to_string();

            let message = item["message"]
                .as_str()
                .or_else(|| item["description"].as_str())
                .unwrap_or("")
                .to_string();

            let file = item["file"]
                .as_str()
                .or_else(|| item["path"].as_str())
                .map(|s| s.to_string());

            let line = item["line"]
                .as_u64()
                .or_else(|| item["lineNumber"].as_u64())
                .map(|n| n as u32);

            // Match fix hints
            let fix_macro = fix_hints.and_then(|hints| {
                hints
                    .iter()
                    .find(|h| name.contains(&h.check_pattern))
                    .map(|h| h.macro_id.clone())
            });

            checks.push(A2Check {
                name,
                status,
                message,
                file,
                line,
                fix_macro,
            });
        }
    } else {
        // Fallback: parse line-by-line
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (status, message) = if trimmed.starts_with("PASS")
                || trimmed.starts_with("✓")
                || trimmed.starts_with("[pass]")
            {
                ("pass".to_string(), trimmed.to_string())
            } else if trimmed.starts_with("FAIL")
                || trimmed.starts_with("✗")
                || trimmed.starts_with("[fail]")
            {
                ("fail".to_string(), trimmed.to_string())
            } else if trimmed.starts_with("WARN") || trimmed.starts_with("[warn]") {
                ("warning".to_string(), trimmed.to_string())
            } else {
                continue;
            };

            checks.push(A2Check {
                name: message.clone(),
                status,
                message,
                file: None,
                line: None,
                fix_macro: None,
            });
        }
    }

    checks
}
