use std::path::Path;

/// Detect the primary project type from filesystem markers.
pub fn detect_project_type(root: &Path) -> Option<String> {
    // Check in order of specificity
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
    None
}
