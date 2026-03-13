use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line_number: u32,
    pub line: String,
}

#[tauri::command]
pub async fn search_project(
    root_path: String,
    query: String,
    file_pattern: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let root = Path::new(&root_path);

    // Prefer ripgrep, fall back to grep
    let (program, base_args) = if which::which("rg").is_ok() {
        ("rg", vec!["--line-number", "--no-heading", "--color=never", "--max-count=200"])
    } else {
        ("grep", vec!["-rn", "--color=never"])
    };

    let mut cmd = Command::new(program);
    for arg in &base_args {
        cmd.arg(arg);
    }

    if let Some(ref pattern) = file_pattern {
        if program == "rg" {
            cmd.arg("--glob").arg(pattern);
        } else {
            cmd.arg("--include").arg(pattern);
        }
    }

    cmd.arg(&query).current_dir(root);

    let output = cmd.output().map_err(|e| format!("Search failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut results = Vec::new();
    for line in stdout.lines() {
        // Format: file:line_number:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let line_num: u32 = parts[1].parse().unwrap_or(0);
            results.push(SearchResult {
                file: parts[0].to_string(),
                line_number: line_num,
                line: parts[2].to_string(),
            });
        }
    }

    Ok(results)
}
