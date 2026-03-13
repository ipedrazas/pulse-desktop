use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDependency {
    pub name: String,
    pub current: String,
    pub wanted: String,
    pub latest: String,
    pub dep_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditVulnerability {
    pub name: String,
    pub severity: String,
    pub title: String,
    pub url: String,
    pub range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthSnapshot {
    pub outdated: Vec<NodeDependency>,
    pub vulnerabilities: Vec<AuditVulnerability>,
    pub outdated_count: usize,
    pub vuln_count: usize,
}

fn detect_package_manager(root: &Path) -> &'static str {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        "bun"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

pub fn get_outdated(root: &Path) -> Result<Vec<NodeDependency>, String> {
    let pm = detect_package_manager(root);

    let output = Command::new(pm)
        .args(["outdated", "--json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run {} outdated: {}", pm, e))?;

    // npm/pnpm outdated returns exit code 1 when there are outdated deps
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut deps = Vec::new();

    match parsed {
        serde_json::Value::Object(map) => {
            // npm/pnpm format: { "package-name": { "current": "...", "wanted": "...", "latest": "..." } }
            for (name, info) in map {
                let current = info["current"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let wanted = info["wanted"].as_str().unwrap_or("").to_string();
                let latest = info["latest"].as_str().unwrap_or("").to_string();
                let dep_type = info["type"]
                    .as_str()
                    .or_else(|| info["dependencyType"].as_str())
                    .unwrap_or("dependencies")
                    .to_string();
                deps.push(NodeDependency {
                    name,
                    current,
                    wanted,
                    latest,
                    dep_type,
                });
            }
        }
        serde_json::Value::Array(arr) => {
            // Some package managers return arrays
            for item in arr {
                let name = item["name"]
                    .as_str()
                    .or_else(|| item["package"].as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let current = item["current"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let wanted = item["wanted"].as_str().unwrap_or("").to_string();
                let latest = item["latest"].as_str().unwrap_or("").to_string();
                deps.push(NodeDependency {
                    name,
                    current,
                    wanted,
                    latest,
                    dep_type: "dependencies".to_string(),
                });
            }
        }
        _ => {}
    }

    Ok(deps)
}

pub fn get_audit(root: &Path) -> Result<Vec<AuditVulnerability>, String> {
    let pm = detect_package_manager(root);

    let output = Command::new(pm)
        .args(["audit", "--json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run {} audit: {}", pm, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut vulns = Vec::new();

    // npm audit JSON can be one object or newline-delimited JSON
    // Try parsing as single object first
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
        // npm format: { "vulnerabilities": { "pkg": { ... } } }
        if let Some(vuln_map) = parsed.get("vulnerabilities").and_then(|v| v.as_object()) {
            for (name, info) in vuln_map {
                let severity = info["severity"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let title = info["title"]
                    .as_str()
                    .or_else(|| info["name"].as_str())
                    .unwrap_or("")
                    .to_string();
                let url = info["url"].as_str().unwrap_or("").to_string();
                let range = info["range"].as_str().unwrap_or("").to_string();
                vulns.push(AuditVulnerability {
                    name: name.clone(),
                    severity,
                    title,
                    url,
                    range,
                });
            }
        }
        // pnpm format: { "advisories": { ... } }
        if let Some(advisories) = parsed.get("advisories").and_then(|v| v.as_object()) {
            for (_id, info) in advisories {
                let name = info["module_name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let severity = info["severity"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let title = info["title"].as_str().unwrap_or("").to_string();
                let url = info["url"].as_str().unwrap_or("").to_string();
                let range = info["vulnerable_versions"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                vulns.push(AuditVulnerability {
                    name,
                    severity,
                    title,
                    url,
                    range,
                });
            }
        }
    }

    Ok(vulns)
}

pub fn get_snapshot(root: &Path) -> Result<NodeHealthSnapshot, String> {
    let outdated = get_outdated(root).unwrap_or_default();
    let vulnerabilities = get_audit(root).unwrap_or_default();
    let outdated_count = outdated.len();
    let vuln_count = vulnerabilities.len();

    Ok(NodeHealthSnapshot {
        outdated,
        vulnerabilities,
        outdated_count,
        vuln_count,
    })
}
