use crate::capabilities::{go, java, node, python, rust_cap};
use crate::capabilities::detect::detect_project_type;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub project_type: Option<String>,
    pub node: Option<node::NodeHealthSnapshot>,
    pub go: Option<go::GoHealthSnapshot>,
    pub python: Option<python::PythonHealthSnapshot>,
    pub rust: Option<rust_cap::RustHealthSnapshot>,
    pub java: Option<java::JavaHealthSnapshot>,
    pub env_parity: Option<EnvParityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvParityResult {
    pub missing_keys: Vec<String>,
    pub extra_keys: Vec<String>,
    pub empty_keys: Vec<String>,
    pub example_count: usize,
    pub actual_count: usize,
}

fn parse_env_keys(path: &Path) -> Vec<(String, bool)> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.is_empty() {
                return None;
            }
            let key = parts[0].trim().to_string();
            let has_value = parts.get(1).map(|v| !v.trim().is_empty()).unwrap_or(false);
            Some((key, has_value))
        })
        .collect()
}

#[tauri::command]
pub async fn get_health_summary(
    pool: State<'_, SqlitePool>,
    project_id: String,
    root_path: String,
) -> Result<HealthSummary, String> {
    let root = Path::new(&root_path);
    let project_type = detect_project_type(root);

    let node_snapshot = if root.join("package.json").exists() {
        Some(node::get_snapshot(root)?)
    } else {
        None
    };

    let go_snapshot = if root.join("go.mod").exists() {
        Some(go::get_snapshot(root)?)
    } else {
        None
    };

    let python_snapshot = if root.join("requirements.txt").exists()
        || root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
    {
        Some(python::get_snapshot(root)?)
    } else {
        None
    };

    let rust_snapshot = if root.join("Cargo.toml").exists() {
        Some(rust_cap::get_snapshot(root)?)
    } else {
        None
    };

    let java_snapshot = if root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists()
    {
        Some(java::get_snapshot(root)?)
    } else {
        None
    };

    let env_parity = check_env_parity(root);

    // Store as snapshot
    let payload = serde_json::json!({
        "node": &node_snapshot,
        "go": &go_snapshot,
        "python": &python_snapshot,
        "rust": &rust_snapshot,
        "java": &java_snapshot,
        "env_parity": &env_parity,
    });

    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let _ = sqlx::query(
        "INSERT INTO snapshots (project_id, kind, commit_sha, payload) VALUES (?, 'health', ?, ?)",
    )
    .bind(&project_id)
    .bind(&commit_sha)
    .bind(payload.to_string())
    .execute(pool.inner())
    .await;

    Ok(HealthSummary {
        project_type,
        node: node_snapshot,
        go: go_snapshot,
        python: python_snapshot,
        rust: rust_snapshot,
        java: java_snapshot,
        env_parity,
    })
}

fn check_env_parity(root: &Path) -> Option<EnvParityResult> {
    let example_path = root.join(".env.example");
    let actual_path = root.join(".env");

    if !example_path.exists() {
        return None;
    }

    let example_keys = parse_env_keys(&example_path);
    let actual_entries = parse_env_keys(&actual_path);

    let example_key_set: std::collections::HashSet<String> =
        example_keys.iter().map(|(k, _)| k.clone()).collect();
    let actual_key_set: std::collections::HashSet<String> =
        actual_entries.iter().map(|(k, _)| k.clone()).collect();

    let missing_keys: Vec<String> = example_key_set
        .difference(&actual_key_set)
        .cloned()
        .collect();
    let extra_keys: Vec<String> = actual_key_set
        .difference(&example_key_set)
        .cloned()
        .collect();
    let empty_keys: Vec<String> = actual_entries
        .iter()
        .filter(|(_, has_val)| !has_val)
        .map(|(k, _)| k.clone())
        .collect();

    Some(EnvParityResult {
        example_count: example_keys.len(),
        actual_count: actual_entries.len(),
        missing_keys,
        extra_keys,
        empty_keys,
    })
}

#[tauri::command]
pub async fn get_env_parity(root_path: String) -> Result<Option<EnvParityResult>, String> {
    Ok(check_env_parity(Path::new(&root_path)))
}

#[tauri::command]
pub async fn get_node_health(root_path: String) -> Result<node::NodeHealthSnapshot, String> {
    node::get_snapshot(Path::new(&root_path))
}

#[tauri::command]
pub async fn get_go_health(root_path: String) -> Result<go::GoHealthSnapshot, String> {
    go::get_snapshot(Path::new(&root_path))
}

#[tauri::command]
pub async fn get_python_health(root_path: String) -> Result<python::PythonHealthSnapshot, String> {
    python::get_snapshot(Path::new(&root_path))
}

#[tauri::command]
pub async fn get_rust_health(root_path: String) -> Result<rust_cap::RustHealthSnapshot, String> {
    rust_cap::get_snapshot(Path::new(&root_path))
}

#[tauri::command]
pub async fn get_java_health(root_path: String) -> Result<java::JavaHealthSnapshot, String> {
    java::get_snapshot(Path::new(&root_path))
}
