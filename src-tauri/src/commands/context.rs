use crate::models::PulseConfig;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
    pub exists: bool,
}

#[derive(serde::Serialize)]
pub struct ContextBundle {
    pub files: Vec<ContextFile>,
    pub git_branch: Option<String>,
    pub git_sha: Option<String>,
    pub git_dirty: Option<bool>,
    pub total_bytes: usize,
}

#[tauri::command]
pub async fn get_context_files(root_path: String) -> Result<ContextBundle, String> {
    let root = Path::new(&root_path);
    let config_path = root.join(".pulse.yaml");

    if !config_path.exists() {
        return Ok(ContextBundle {
            files: vec![],
            git_branch: None,
            git_sha: None,
            git_dirty: None,
            total_bytes: 0,
        });
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: PulseConfig = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    let mut total_bytes = 0;

    if let Some(ctx) = &config.context {
        if let Some(file_list) = &ctx.files {
            for file_path in file_list {
                let full_path = root.join(file_path);
                let (file_content, exists) = if full_path.exists() {
                    let c = std::fs::read_to_string(&full_path).unwrap_or_default();
                    total_bytes += c.len();
                    (c, true)
                } else {
                    (String::new(), false)
                };

                files.push(ContextFile {
                    path: file_path.clone(),
                    content: file_content,
                    exists,
                });
            }
        }
    }

    // Git info
    let git_branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let git_dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty());

    Ok(ContextBundle {
        files,
        git_branch,
        git_sha,
        git_dirty,
        total_bytes,
    })
}

#[tauri::command]
pub async fn build_context_string(root_path: String) -> Result<String, String> {
    let bundle = get_context_files(root_path).await?;
    let mut output = String::new();

    // Git header
    if let Some(ref branch) = bundle.git_branch {
        output.push_str(&format!("Branch: {}", branch));
        if let Some(ref sha) = bundle.git_sha {
            output.push_str(&format!(" ({})", sha));
        }
        if let Some(dirty) = bundle.git_dirty {
            if dirty {
                output.push_str(" [dirty]");
            }
        }
        output.push('\n');
        output.push_str("---\n\n");
    }

    for file in &bundle.files {
        if file.exists {
            output.push_str(&format!("## {}\n\n", file.path));
            output.push_str(&file.content);
            output.push_str("\n\n");
        }
    }

    Ok(output)
}
