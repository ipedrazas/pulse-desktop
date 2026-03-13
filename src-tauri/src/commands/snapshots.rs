use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Snapshot {
    pub id: i64,
    pub project_id: String,
    pub kind: String,
    pub commit_sha: Option<String>,
    pub payload: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub kind: String,
    pub older: SnapshotSummary,
    pub newer: SnapshotSummary,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: i64,
    pub commit_sha: Option<String>,
    pub created_at: String,
    pub item_count: usize,
}

#[tauri::command]
pub async fn list_snapshots(
    pool: State<'_, SqlitePool>,
    project_id: String,
    kind: Option<String>,
) -> Result<Vec<Snapshot>, String> {
    let snapshots: Vec<Snapshot> = if let Some(k) = kind {
        sqlx::query_as(
            "SELECT * FROM snapshots WHERE project_id = ? AND kind = ? ORDER BY created_at DESC LIMIT 20",
        )
        .bind(&project_id)
        .bind(&k)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT * FROM snapshots WHERE project_id = ? ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&project_id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(snapshots)
}

#[tauri::command]
pub async fn diff_snapshots(
    pool: State<'_, SqlitePool>,
    older_id: i64,
    newer_id: i64,
) -> Result<SnapshotDiff, String> {
    let older: Snapshot = sqlx::query_as("SELECT * FROM snapshots WHERE id = ?")
        .bind(older_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let newer: Snapshot = sqlx::query_as("SELECT * FROM snapshots WHERE id = ?")
        .bind(newer_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let older_keys = extract_keys(&older.payload);
    let newer_keys = extract_keys(&newer.payload);

    let older_set: std::collections::HashSet<&str> = older_keys.iter().map(|s| s.as_str()).collect();
    let newer_set: std::collections::HashSet<&str> = newer_keys.iter().map(|s| s.as_str()).collect();

    let added: Vec<String> = newer_set
        .difference(&older_set)
        .map(|s| s.to_string())
        .collect();
    let removed: Vec<String> = older_set
        .difference(&newer_set)
        .map(|s| s.to_string())
        .collect();

    // Detect changed items by comparing values in both
    let changed = detect_changed_items(&older.payload, &newer.payload);

    Ok(SnapshotDiff {
        kind: newer.kind.clone(),
        older: SnapshotSummary {
            id: older.id,
            commit_sha: older.commit_sha,
            created_at: older.created_at,
            item_count: older_keys.len(),
        },
        newer: SnapshotSummary {
            id: newer.id,
            commit_sha: newer.commit_sha,
            created_at: newer.created_at,
            item_count: newer_keys.len(),
        },
        added,
        removed,
        changed,
    })
}

fn extract_keys(payload: &str) -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        collect_dep_names(&value, &mut keys);
    }
    keys
}

fn collect_dep_names(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            // Look for node outdated items
            if let Some(node) = map.get("node") {
                if let Some(outdated) = node.get("outdated").and_then(|v| v.as_array()) {
                    for item in outdated {
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            keys.push(name.to_string());
                        }
                    }
                }
                if let Some(vulns) = node.get("vulnerabilities").and_then(|v| v.as_array()) {
                    for item in vulns {
                        if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                            keys.push(format!("vuln:{}", name));
                        }
                    }
                }
            }
            // Go modules
            if let Some(go) = map.get("go") {
                if let Some(modules) = go.get("modules").and_then(|v| v.as_array()) {
                    for item in modules {
                        if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                            keys.push(path.to_string());
                        }
                    }
                }
                if let Some(vulns) = go.get("vulnerabilities").and_then(|v| v.as_array()) {
                    for item in vulns {
                        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                            keys.push(format!("vuln:{}", id));
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    keys.push(name.to_string());
                } else if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                    keys.push(path.to_string());
                }
            }
        }
        _ => {}
    }
}

fn detect_changed_items(older: &str, newer: &str) -> Vec<String> {
    let mut changed = Vec::new();

    let older_val: serde_json::Value = serde_json::from_str(older).unwrap_or_default();
    let newer_val: serde_json::Value = serde_json::from_str(newer).unwrap_or_default();

    // Compare node outdated versions
    if let (Some(old_outdated), Some(new_outdated)) = (
        older_val
            .pointer("/node/outdated")
            .and_then(|v| v.as_array()),
        newer_val
            .pointer("/node/outdated")
            .and_then(|v| v.as_array()),
    ) {
        for new_item in new_outdated {
            let name = new_item["name"].as_str().unwrap_or("");
            let new_latest = new_item["latest"].as_str().unwrap_or("");

            if let Some(old_item) = old_outdated.iter().find(|i| i["name"].as_str() == Some(name))
            {
                let old_latest = old_item["latest"].as_str().unwrap_or("");
                if old_latest != new_latest && !name.is_empty() {
                    changed.push(format!("{}: {} -> {}", name, old_latest, new_latest));
                }
            }
        }
    }

    changed
}
