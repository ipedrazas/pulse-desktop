use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    pub size: Option<u64>,
}

#[tauri::command]
pub async fn get_file_tree(
    root_path: String,
    max_depth: Option<usize>,
) -> Result<Vec<FileNode>, String> {
    let root = Path::new(&root_path);
    if !root.is_dir() {
        return Err("Not a directory".to_string());
    }

    let depth = max_depth.unwrap_or(4);
    let mut nodes: Vec<FileNode> = Vec::new();

    let walker = WalkBuilder::new(root)
        .max_depth(Some(depth))
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    // Collect entries, skip the root itself
    let mut entries: Vec<(String, bool, u64)> = Vec::new();
    for entry in walker.flatten() {
        let path = entry.path();
        if path == root {
            continue;
        }

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let is_dir = path.is_dir();
        let size = if !is_dir {
            path.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        entries.push((rel_path, is_dir, size));
    }

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| {
        if a.1 != b.1 {
            return b.1.cmp(&a.1); // dirs first
        }
        a.0.cmp(&b.0)
    });

    // Build flat list (tree building done on frontend for performance)
    for (rel_path, is_dir, size) in entries {
        let name = rel_path
            .rsplit('/')
            .next()
            .unwrap_or(&rel_path)
            .to_string();

        nodes.push(FileNode {
            name,
            path: rel_path,
            is_dir,
            children: None,
            size: if is_dir { None } else { Some(size) },
        });
    }

    Ok(nodes)
}

#[tauri::command]
pub async fn read_file_content(
    root_path: String,
    file_path: String,
    max_bytes: Option<usize>,
) -> Result<String, String> {
    let full_path = Path::new(&root_path).join(&file_path);

    if !full_path.exists() {
        return Err("File not found".to_string());
    }

    if full_path.is_dir() {
        return Err("Path is a directory".to_string());
    }

    let metadata = full_path.metadata().map_err(|e| e.to_string())?;
    let limit = max_bytes.unwrap_or(512_000); // 500KB default

    if metadata.len() as usize > limit {
        return Err(format!(
            "File too large ({} bytes, limit {})",
            metadata.len(),
            limit
        ));
    }

    std::fs::read_to_string(&full_path).map_err(|_| {
        // Binary file — return a placeholder
        "Binary file — cannot display".to_string()
    })
}
