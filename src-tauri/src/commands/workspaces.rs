use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceWithProjects {
    pub workspace: Workspace,
    pub project_ids: Vec<String>,
}

#[tauri::command]
pub async fn list_workspaces(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<WorkspaceWithProjects>, String> {
    let workspaces: Vec<Workspace> = sqlx::query_as("SELECT * FROM workspaces ORDER BY name")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for ws in workspaces {
        let project_ids: Vec<(String,)> =
            sqlx::query_as("SELECT project_id FROM workspace_projects WHERE workspace_id = ?")
                .bind(&ws.id)
                .fetch_all(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

        results.push(WorkspaceWithProjects {
            workspace: ws,
            project_ids: project_ids.into_iter().map(|(id,)| id).collect(),
        });
    }

    Ok(results)
}

#[tauri::command]
pub async fn create_workspace(
    pool: State<'_, SqlitePool>,
    name: String,
    description: Option<String>,
) -> Result<Workspace, String> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO workspaces (id, name, description) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(&description)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let ws: Workspace = sqlx::query_as("SELECT * FROM workspaces WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ws)
}

#[tauri::command]
pub async fn delete_workspace(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM workspace_projects WHERE workspace_id = ?")
        .bind(&workspace_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM workspaces WHERE id = ?")
        .bind(&workspace_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn add_project_to_workspace(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    project_id: String,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO workspace_projects (workspace_id, project_id) VALUES (?, ?)",
    )
    .bind(&workspace_id)
    .bind(&project_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_project_from_workspace(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    project_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM workspace_projects WHERE workspace_id = ? AND project_id = ?")
        .bind(&workspace_id)
        .bind(&project_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
