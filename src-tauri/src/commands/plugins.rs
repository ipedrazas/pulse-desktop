use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub entry_point: String,
    pub plugin_type: String,
    pub enabled: i32,
    pub config: Option<String>,
    pub installed_at: String,
}

#[tauri::command]
pub async fn list_plugins(pool: State<'_, SqlitePool>) -> Result<Vec<Plugin>, String> {
    let plugins: Vec<Plugin> =
        sqlx::query_as("SELECT * FROM plugins ORDER BY name")
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    Ok(plugins)
}

#[tauri::command]
pub async fn register_plugin(
    pool: State<'_, SqlitePool>,
    name: String,
    description: Option<String>,
    version: Option<String>,
    entry_point: String,
    plugin_type: String,
    config: Option<String>,
) -> Result<Plugin, String> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO plugins (id, name, description, version, entry_point, plugin_type, config)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .bind(&version)
    .bind(&entry_point)
    .bind(&plugin_type)
    .bind(&config)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let plugin: Plugin = sqlx::query_as("SELECT * FROM plugins WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(plugin)
}

#[tauri::command]
pub async fn toggle_plugin(
    pool: State<'_, SqlitePool>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    sqlx::query("UPDATE plugins SET enabled = ? WHERE id = ?")
        .bind(enabled as i32)
        .bind(&plugin_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn remove_plugin(
    pool: State<'_, SqlitePool>,
    plugin_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&plugin_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
