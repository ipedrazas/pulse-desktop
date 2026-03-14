use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustDependency {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustAuditVuln {
    pub id: String,
    pub package: String,
    pub title: String,
    pub severity: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustHealthSnapshot {
    pub outdated: Vec<RustDependency>,
    pub vulnerabilities: Vec<RustAuditVuln>,
    pub outdated_count: usize,
    pub vuln_count: usize,
    pub rust_version: Option<String>,
}

pub fn get_outdated(root: &Path) -> Result<Vec<RustDependency>, String> {
    // cargo-outdated if available, otherwise skip
    if which::which("cargo-outdated").is_err() {
        // Try cargo update --dry-run as fallback
        let output = Command::new("cargo")
            .args(["update", "--dry-run"])
            .current_dir(root)
            .output()
            .map_err(|e| format!("Failed to run cargo: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut deps = Vec::new();

        // Parse "Updating crate_name v0.1.0 -> v0.2.0" lines
        for line in stderr.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Updating") && trimmed.contains("->") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    deps.push(RustDependency {
                        name: parts[1].to_string(),
                        current: parts[2].trim_start_matches('v').to_string(),
                        latest: parts[4].trim_start_matches('v').to_string(),
                        kind: "normal".to_string(),
                    });
                }
            }
        }

        return Ok(deps);
    }

    let output = Command::new("cargo")
        .args(["outdated", "--format=json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run cargo outdated: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut deps = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(crates) = parsed.get("dependencies").and_then(|d| d.as_array()) {
            for cr in crates {
                deps.push(RustDependency {
                    name: cr["name"].as_str().unwrap_or("").to_string(),
                    current: cr["project"].as_str().unwrap_or("").to_string(),
                    latest: cr["latest"].as_str().unwrap_or("").to_string(),
                    kind: cr["kind"].as_str().unwrap_or("normal").to_string(),
                });
            }
        }
    }

    Ok(deps)
}

pub fn get_audit(root: &Path) -> Result<Vec<RustAuditVuln>, String> {
    if which::which("cargo-audit").is_err() {
        return Ok(vec![]);
    }

    let output = Command::new("cargo")
        .args(["audit", "--json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run cargo audit: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut vulns = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(vuln_list) = parsed
            .get("vulnerabilities")
            .and_then(|v| v.get("list"))
            .and_then(|l| l.as_array())
        {
            for v in vuln_list {
                let advisory = &v["advisory"];
                vulns.push(RustAuditVuln {
                    id: advisory["id"].as_str().unwrap_or("").to_string(),
                    package: v["package"]["name"].as_str().unwrap_or("").to_string(),
                    title: advisory["title"].as_str().unwrap_or("").to_string(),
                    severity: advisory["cvss"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                    url: advisory["url"].as_str().unwrap_or("").to_string(),
                });
            }
        }
    }

    Ok(vulns)
}

pub fn get_snapshot(root: &Path) -> Result<RustHealthSnapshot, String> {
    let rust_version = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let outdated = get_outdated(root).unwrap_or_default();
    let vulnerabilities = get_audit(root).unwrap_or_default();
    let outdated_count = outdated.len();
    let vuln_count = vulnerabilities.len();

    Ok(RustHealthSnapshot {
        outdated,
        vulnerabilities,
        outdated_count,
        vuln_count,
        rust_version,
    })
}
