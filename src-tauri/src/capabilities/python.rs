use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonDependency {
    pub name: String,
    pub current: String,
    pub latest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonHealthSnapshot {
    pub outdated: Vec<PythonDependency>,
    pub vulnerabilities: Vec<PythonVulnerability>,
    pub outdated_count: usize,
    pub vuln_count: usize,
    pub python_version: Option<String>,
    pub venv_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonVulnerability {
    pub package: String,
    pub vulnerability_id: String,
    pub description: String,
    pub fixed_in: String,
}

fn detect_python_cmd(root: &Path) -> &'static str {
    // Check for virtual environment
    if root.join(".venv/bin/python").exists() || root.join("venv/bin/python").exists() {
        if root.join(".venv/bin/python").exists() {
            return ".venv/bin/python";
        }
        return "venv/bin/python";
    }
    "python3"
}

fn detect_pip_cmd(root: &Path) -> &'static str {
    if root.join(".venv/bin/pip").exists() {
        return ".venv/bin/pip";
    }
    if root.join("venv/bin/pip").exists() {
        return "venv/bin/pip";
    }
    "pip3"
}

pub fn get_outdated(root: &Path) -> Result<Vec<PythonDependency>, String> {
    let pip = detect_pip_cmd(root);
    let output = Command::new(pip)
        .args(["list", "--outdated", "--format=json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run pip: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let items: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).unwrap_or_default();

    Ok(items
        .iter()
        .map(|item| PythonDependency {
            name: item["name"].as_str().unwrap_or("").to_string(),
            current: item["version"].as_str().unwrap_or("").to_string(),
            latest: item["latest_version"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

pub fn get_audit(root: &Path) -> Result<Vec<PythonVulnerability>, String> {
    // Try pip-audit if available
    if which::which("pip-audit").is_err() {
        return Ok(vec![]);
    }

    let output = Command::new("pip-audit")
        .args(["--format=json", "--desc"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run pip-audit: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut vulns = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_vulns) = dep.get("vulns").and_then(|v| v.as_array()) {
                    let pkg = dep["name"].as_str().unwrap_or("").to_string();
                    for vuln in dep_vulns {
                        vulns.push(PythonVulnerability {
                            package: pkg.clone(),
                            vulnerability_id: vuln["id"].as_str().unwrap_or("").to_string(),
                            description: vuln["description"].as_str().unwrap_or("").to_string(),
                            fixed_in: vuln["fix_versions"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }

    Ok(vulns)
}

pub fn get_snapshot(root: &Path) -> Result<PythonHealthSnapshot, String> {
    let python = detect_python_cmd(root);
    let python_version = Command::new(python)
        .args(["--version"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let venv_detected =
        root.join(".venv").exists() || root.join("venv").exists();

    let outdated = get_outdated(root).unwrap_or_default();
    let vulnerabilities = get_audit(root).unwrap_or_default();
    let outdated_count = outdated.len();
    let vuln_count = vulnerabilities.len();

    Ok(PythonHealthSnapshot {
        outdated,
        vulnerabilities,
        outdated_count,
        vuln_count,
        python_version,
        venv_detected,
    })
}
