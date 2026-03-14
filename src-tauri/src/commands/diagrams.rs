use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramEntry {
    pub title: String,
    pub source_file: Option<String>,
    pub mermaid: String,
}

/// Scan markdown files in the project for ```mermaid blocks.
#[tauri::command]
pub async fn extract_mermaid_diagrams(root_path: String) -> Result<Vec<DiagramEntry>, String> {
    let root = Path::new(&root_path);
    let mut diagrams = Vec::new();

    let walker = WalkBuilder::new(root)
        .max_depth(Some(6))
        .hidden(true)
        .git_ignore(true)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | "node_modules" | "target" | "__pycache__" | "dist" | "vendor"
            )
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "mdx" {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) if c.len() < 1_000_000 => c,
            _ => continue,
        };

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let mut block_idx = 0;
        let mut in_block = false;
        let mut current_block = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if !in_block && (trimmed == "```mermaid" || trimmed.starts_with("```mermaid ")) {
                in_block = true;
                current_block.clear();
                continue;
            }
            if in_block {
                if trimmed == "```" {
                    in_block = false;
                    block_idx += 1;
                    let block_content = current_block.trim().to_string();
                    if !block_content.is_empty() {
                        // Try to derive a title from the first line of the block
                        let title = derive_title(&block_content, &rel_path, block_idx);
                        diagrams.push(DiagramEntry {
                            title,
                            source_file: Some(rel_path.clone()),
                            mermaid: block_content,
                        });
                    }
                } else {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }
        }
    }

    Ok(diagrams)
}

fn derive_title(mermaid_content: &str, file_path: &str, index: usize) -> String {
    let first_line = mermaid_content.lines().next().unwrap_or("").trim();

    // Extract diagram type from first line
    let diagram_type = if first_line.starts_with("graph") || first_line.starts_with("flowchart") {
        "Flowchart"
    } else if first_line.starts_with("sequenceDiagram") {
        "Sequence Diagram"
    } else if first_line.starts_with("classDiagram") {
        "Class Diagram"
    } else if first_line.starts_with("erDiagram") {
        "ER Diagram"
    } else if first_line.starts_with("stateDiagram") {
        "State Diagram"
    } else if first_line.starts_with("gantt") {
        "Gantt Chart"
    } else if first_line.starts_with("pie") {
        "Pie Chart"
    } else if first_line.starts_with("gitGraph") || first_line.starts_with("gitgraph") {
        "Git Graph"
    } else if first_line.starts_with("C4") {
        "C4 Diagram"
    } else {
        "Diagram"
    };

    // Use filename without extension as context
    let file_stem = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);

    if index <= 1 {
        format!("{} — {}", diagram_type, file_stem)
    } else {
        format!("{} #{} — {}", diagram_type, index, file_stem)
    }
}

/// Generate an ER diagram from Pulse's internal database schema.
#[tauri::command]
pub async fn generate_db_diagram(
    pool: State<'_, SqlitePool>,
) -> Result<DiagramEntry, String> {
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

    Ok(DiagramEntry {
        title: "Pulse DB Schema".to_string(),
        source_file: None,
        mermaid: lines.join("\n"),
    })
}

/// Save a mermaid diagram to a file in the project.
#[tauri::command]
pub async fn save_diagram_to_file(
    root_path: String,
    file_name: String,
    mermaid_content: String,
) -> Result<String, String> {
    let root = Path::new(&root_path);

    // Sanitize filename
    let safe_name = file_name
        .replace(['/', '\\'], "_")
        .replace("..", "_")
        .trim()
        .to_string();

    if safe_name.is_empty() {
        return Err("Invalid file name".to_string());
    }

    // Ensure it has an .md extension
    let final_name = if safe_name.ends_with(".md") {
        safe_name
    } else if safe_name.ends_with(".mmd") || safe_name.ends_with(".mermaid") {
        safe_name
    } else {
        format!("{}.md", safe_name)
    };

    let file_path = root.join(&final_name);

    // Wrap in markdown mermaid block if saving as .md
    let content = if final_name.ends_with(".md") {
        format!("```mermaid\n{}\n```\n", mermaid_content)
    } else {
        format!("{}\n", mermaid_content)
    };

    std::fs::write(&file_path, content).map_err(|e| format!("Failed to save: {}", e))?;

    Ok(final_name)
}
