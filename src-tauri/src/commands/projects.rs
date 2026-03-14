use crate::capabilities::detect::{detect_project_type, read_a2_languages};
use crate::models::{GitInfo, Project, ProjectInfo, PulseConfig};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::Path;
use tauri::State;

fn make_project_id(root_path: &str, remote_url: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_path.as_bytes());
    if let Some(url) = remote_url {
        hasher.update(url.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn get_git_info(root: &Path) -> Option<GitInfo> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return None;
    }

    let sha_output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    let sha = String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string();

    let dirty_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()?;
    let dirty = !dirty_output.stdout.is_empty();

    Some(GitInfo { branch, sha, dirty })
}

fn get_remote_url(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .ok()?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

#[tauri::command]
pub async fn open_project(pool: State<'_, SqlitePool>, path: String) -> Result<ProjectInfo, String> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let remote_url = get_remote_url(root);
    let id = make_project_id(&path, remote_url.as_deref());

    // Upsert project
    sqlx::query(
        "INSERT INTO projects (id, name, root_path, remote_url, last_opened)
         VALUES (?, ?, ?, ?, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET last_opened = datetime('now'), name = excluded.name",
    )
    .bind(&id)
    .bind(&name)
    .bind(&path)
    .bind(&remote_url)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let project = Project {
        id,
        name,
        root_path: path.clone(),
        remote_url,
        created_at: String::new(),
        last_opened: String::new(),
    };

    let project_type = detect_project_type(root);
    let git = get_git_info(root);
    let has_pulse_yaml = root.join(".pulse.yaml").exists();
    let has_a2_yaml = root.join(".a2.yaml").exists();

    let (languages, language_dirs) = match read_a2_languages(root) {
        Some(info) => (
            Some(info.languages),
            if info.source_dirs.is_empty() { None } else { Some(info.source_dirs) },
        ),
        None => (None, None),
    };

    Ok(ProjectInfo {
        project,
        project_type,
        languages,
        language_dirs,
        git,
        has_pulse_yaml,
        has_a2_yaml,
    })
}

#[tauri::command]
pub async fn list_projects(pool: State<'_, SqlitePool>) -> Result<Vec<Project>, String> {
    let projects: Vec<Project> =
        sqlx::query_as("SELECT * FROM projects ORDER BY last_opened DESC")
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    Ok(projects)
}

#[tauri::command]
pub async fn get_project_info(
    pool: State<'_, SqlitePool>,
    project_id: String,
) -> Result<ProjectInfo, String> {
    let project: Project =
        sqlx::query_as("SELECT * FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    let root = Path::new(&project.root_path);
    let project_type = detect_project_type(root);
    let git = get_git_info(root);
    let has_pulse_yaml = root.join(".pulse.yaml").exists();
    let has_a2_yaml = root.join(".a2.yaml").exists();

    let (languages, language_dirs) = match read_a2_languages(root) {
        Some(info) => (
            Some(info.languages),
            if info.source_dirs.is_empty() { None } else { Some(info.source_dirs) },
        ),
        None => (None, None),
    };

    Ok(ProjectInfo {
        project,
        project_type,
        languages,
        language_dirs,
        git,
        has_pulse_yaml,
        has_a2_yaml,
    })
}

#[tauri::command]
pub async fn remove_project(pool: State<'_, SqlitePool>, project_id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_pulse_config(root_path: String) -> Result<Option<PulseConfig>, String> {
    let config_path = Path::new(&root_path).join(".pulse.yaml");
    if !config_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: PulseConfig = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(config))
}

#[tauri::command]
pub async fn get_git_status(root_path: String) -> Result<Option<GitInfo>, String> {
    Ok(get_git_info(Path::new(&root_path)))
}

// --- Worklog ---

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AgentSession {
    pub id: i64,
    pub project_id: String,
    pub title: Option<String>,
    pub tool: Option<String>,
    pub task_summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn get_worklog(
    pool: State<'_, SqlitePool>,
    project_id: String,
) -> Result<Vec<AgentSession>, String> {
    let sessions: Vec<AgentSession> = sqlx::query_as(
        "SELECT * FROM agent_sessions WHERE project_id = ? ORDER BY updated_at DESC LIMIT 10",
    )
    .bind(&project_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(sessions)
}

#[tauri::command]
pub async fn save_worklog_entry(
    pool: State<'_, SqlitePool>,
    project_id: String,
    title: Option<String>,
    tool: Option<String>,
    task_summary: Option<String>,
) -> Result<i64, String> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO agent_sessions (project_id, title, tool, task_summary)
         VALUES (?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&project_id)
    .bind(&title)
    .bind(&tool)
    .bind(&task_summary)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(id)
}
