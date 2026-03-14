use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    pub kind: String,
    pub mermaid: String,
}

#[tauri::command]
pub async fn generate_folder_diagram(root_path: String) -> Result<DiagramResult, String> {
    let root = Path::new(&root_path);
    let mut lines = vec!["graph TD".to_string()];
    let mut id_counter = 0;

    fn walk_dir(
        dir: &Path,
        root: &Path,
        parent_id: Option<usize>,
        lines: &mut Vec<String>,
        counter: &mut usize,
        depth: usize,
    ) {
        if depth > 3 {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == "__pycache__" || name == "dist" {
                continue;
            }

            let current_id = *counter;
            *counter += 1;

            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path().as_path())
                .to_string_lossy()
                .to_string();

            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let label = if is_dir {
                format!("    N{}[\"{}/ \"]", current_id, name)
            } else {
                format!("    N{}[\"{}]\"", current_id, name)
            };
            // Use proper Mermaid syntax
            if is_dir {
                lines.push(format!("    N{}[\"{}/\"]", current_id, name));
            } else {
                lines.push(format!("    N{}[\"{}\" ]", current_id, name));
            }

            if let Some(pid) = parent_id {
                lines.push(format!("    N{} --> N{}", pid, current_id));
            }

            if is_dir {
                walk_dir(&entry.path(), root, Some(current_id), lines, counter, depth + 1);
            }

            let _ = (label, rel); // suppress warnings
        }
    }

    walk_dir(root, root, None, &mut lines, &mut id_counter, 0);

    Ok(DiagramResult {
        kind: "folder_structure".to_string(),
        mermaid: lines.join("\n"),
    })
}

#[tauri::command]
pub async fn generate_db_diagram(
    pool: State<'_, SqlitePool>,
) -> Result<DiagramResult, String> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT m.name as table_name, p.name as col_name, p.type as col_type
         FROM sqlite_master m
         JOIN pragma_table_info(m.name) p
         WHERE m.type = 'table' AND m.name NOT LIKE 'sqlite_%'
         ORDER BY m.name, p.cid",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut lines = vec!["erDiagram".to_string()];
    let mut current_table = String::new();

    for (table, col, col_type) in &rows {
        if *table != current_table {
            if !current_table.is_empty() {
                lines.push("    }".to_string());
            }
            lines.push(format!("    {} {{", table));
            current_table = table.clone();
        }
        let mermaid_type = match col_type.to_uppercase().as_str() {
            "TEXT" => "string",
            "INTEGER" => "int",
            "REAL" => "float",
            _ => "string",
        };
        lines.push(format!("        {} {}", mermaid_type, col));
    }

    if !current_table.is_empty() {
        lines.push("    }".to_string());
    }

    Ok(DiagramResult {
        kind: "db_schema".to_string(),
        mermaid: lines.join("\n"),
    })
}
