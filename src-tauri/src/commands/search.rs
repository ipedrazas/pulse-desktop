use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use std::process::Command;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line_number: u32,
    pub line: String,
    pub project_name: Option<String>,
    pub project_id: Option<String>,
}

#[tauri::command]
pub async fn search_project(
    root_path: String,
    query: String,
    file_pattern: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    search_in_directory(&root_path, &query, file_pattern.as_deref(), None, None)
}

#[tauri::command]
pub async fn search_all_projects(
    pool: State<'_, SqlitePool>,
    query: String,
    file_pattern: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let projects: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, root_path FROM projects ORDER BY last_opened DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut all_results = Vec::new();

    for (project_id, project_name, root_path) in projects {
        if !Path::new(&root_path).is_dir() {
            continue;
        }
        let results = search_in_directory(
            &root_path,
            &query,
            file_pattern.as_deref(),
            Some(project_name),
            Some(project_id),
        )?;
        all_results.extend(results);

        // Cap total results
        if all_results.len() >= 500 {
            all_results.truncate(500);
            break;
        }
    }

    Ok(all_results)
}

fn search_in_directory(
    root_path: &str,
    query: &str,
    file_pattern: Option<&str>,
    project_name: Option<String>,
    project_id: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let root = Path::new(root_path);

    let (program, base_args) = if which::which("rg").is_ok() {
        (
            "rg",
            vec![
                "--line-number",
                "--no-heading",
                "--color=never",
                "--max-count=200",
            ],
        )
    } else {
        ("grep", vec!["-rn", "--color=never"])
    };

    let mut cmd = Command::new(program);
    for arg in &base_args {
        cmd.arg(arg);
    }

    if let Some(pattern) = file_pattern {
        if program == "rg" {
            cmd.arg("--glob").arg(pattern);
        } else {
            cmd.arg("--include").arg(pattern);
        }
    }

    cmd.arg(query).current_dir(root);

    let output = cmd.output().map_err(|e| format!("Search failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut results = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let line_num: u32 = parts[1].parse().unwrap_or(0);
            results.push(SearchResult {
                file: parts[0].to_string(),
                line_number: line_num,
                line: parts[2].to_string(),
                project_name: project_name.clone(),
                project_id: project_id.clone(),
            });
        }
    }

    Ok(results)
}
