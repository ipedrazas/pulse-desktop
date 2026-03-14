use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

// --- .a2.yaml partial schema for language detection ---

#[derive(Debug, Deserialize)]
struct A2Config {
    language: Option<A2Language>,
}

#[derive(Debug, Deserialize)]
struct A2Language {
    explicit: Option<Vec<String>>,
    #[serde(flatten)]
    langs: BTreeMap<String, serde_yaml::Value>,
}

/// Parsed language info from .a2.yaml.
#[derive(Debug, Clone)]
pub struct A2LanguageInfo {
    pub languages: Vec<String>,
    pub source_dirs: BTreeMap<String, Vec<String>>,
}

/// Try to read language info from .a2.yaml.
pub fn read_a2_languages(root: &Path) -> Option<A2LanguageInfo> {
    let a2_path = root.join(".a2.yaml");
    let content = std::fs::read_to_string(a2_path).ok()?;
    let config: A2Config = serde_yaml::from_str(&content).ok()?;
    let lang = config.language?;

    let languages = lang.explicit.clone().unwrap_or_default();
    let mut source_dirs: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (key, value) in &lang.langs {
        // Skip the "explicit" key which is already handled
        if key == "explicit" {
            continue;
        }
        let dirs = extract_source_dirs(value);
        if !dirs.is_empty() {
            source_dirs.insert(key.clone(), dirs);
        }
    }

    if languages.is_empty() && source_dirs.is_empty() {
        return None;
    }

    Some(A2LanguageInfo {
        languages,
        source_dirs,
    })
}

fn extract_source_dirs(value: &serde_yaml::Value) -> Vec<String> {
    let mut dirs = Vec::new();

    let source_dir = match value.get("source_dir") {
        Some(sd) => sd,
        None => return dirs,
    };

    match source_dir {
        // Simple list: [ui]
        serde_yaml::Value::Sequence(seq) => {
            for item in seq {
                match item {
                    // String entry
                    serde_yaml::Value::String(s) => dirs.push(s.clone()),
                    // Object entry with "path" key: { path: api, profile: api, ... }
                    serde_yaml::Value::Mapping(map) => {
                        if let Some(serde_yaml::Value::String(p)) =
                            map.get(&serde_yaml::Value::String("path".to_string()))
                        {
                            dirs.push(p.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        // Single string
        serde_yaml::Value::String(s) => dirs.push(s.clone()),
        _ => {}
    }

    dirs
}

/// Detect the primary project type from filesystem markers.
/// Prefers .a2.yaml language declarations when available.
pub fn detect_project_type(root: &Path) -> Option<String> {
    // 1. Try .a2.yaml first — it's the source of truth
    if let Some(a2_info) = read_a2_languages(root) {
        if !a2_info.languages.is_empty() {
            if a2_info.languages.len() == 1 {
                return Some(a2_info.languages[0].clone());
            }
            return Some(format!("monorepo ({})", a2_info.languages.join(", ")));
        }
    }

    // 2. Filesystem-based monorepo detection
    let sub_types = detect_sub_project_types(root);
    if sub_types.len() > 1 {
        return Some(format!("monorepo ({})", sub_types.join(", ")));
    }

    // Monorepo markers without multiple detected sub-types
    if root.join("go.work").exists()
        || root.join("pnpm-workspace.yaml").exists()
        || root.join("lerna.json").exists()
    {
        if sub_types.len() == 1 {
            return Some(format!("monorepo ({})", sub_types[0]));
        }
        return Some("monorepo".to_string());
    }

    // 3. Single-project detection
    if root.join("package.json").exists() {
        if root.join("tsconfig.json").exists() {
            return Some("node-ts".to_string());
        }
        return Some("node".to_string());
    }
    if root.join("go.mod").exists() {
        return Some("go".to_string());
    }
    if root.join("Cargo.toml").exists() {
        return Some("rust".to_string());
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        return Some("python".to_string());
    }
    if root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists()
    {
        return Some("java".to_string());
    }
    None
}

/// Scan immediate subdirectories for project type markers to detect monorepos.
fn detect_sub_project_types(root: &Path) -> Vec<String> {
    let mut types = std::collections::BTreeSet::new();

    // Check root-level markers
    if root.join("go.work").exists() || root.join("go.mod").exists() {
        types.insert("go");
    }
    if root.join("package.json").exists() || root.join("pnpm-workspace.yaml").exists() {
        if root.join("tsconfig.json").exists() {
            types.insert("node-ts");
        } else {
            types.insert("node");
        }
    }
    if root.join("Cargo.toml").exists() {
        types.insert("rust");
    }

    // Scan one level of subdirectories
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return types.into_iter().map(|s| s.to_string()).collect(),
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.')
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == "dist"
            || name_str == "vendor"
        {
            continue;
        }

        if dir.join("go.mod").exists() {
            types.insert("go");
        }
        if dir.join("package.json").exists() {
            if dir.join("tsconfig.json").exists() {
                types.insert("node-ts");
            } else {
                types.insert("node");
            }
        }
        if dir.join("Cargo.toml").exists() {
            types.insert("rust");
        }
        if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() {
            types.insert("python");
        }
        if dir.join("pom.xml").exists()
            || dir.join("build.gradle").exists()
            || dir.join("build.gradle.kts").exists()
        {
            types.insert("java");
        }
    }

    types.into_iter().map(|s| s.to_string()).collect()
}
