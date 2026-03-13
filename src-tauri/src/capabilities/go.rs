use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoModule {
    pub path: String,
    pub version: String,
    pub update_version: Option<String>,
    pub indirect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoVulnerability {
    pub id: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub module_path: String,
    pub found_version: String,
    pub fixed_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoHealthSnapshot {
    pub modules: Vec<GoModule>,
    pub vulnerabilities: Vec<GoVulnerability>,
    pub outdated_count: usize,
    pub vuln_count: usize,
}

pub fn get_modules(root: &Path) -> Result<Vec<GoModule>, String> {
    let output = Command::new("go")
        .args(["list", "-m", "-u", "-json", "all"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run go list: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut modules = Vec::new();

    // go list -json outputs concatenated JSON objects (not an array)
    // We need to parse them one by one
    let mut decoder = stdout.as_ref();
    while !decoder.trim().is_empty() {
        // Find the end of the current JSON object
        let mut brace_count = 0;
        let mut end_idx = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in decoder.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_idx = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        if end_idx == 0 {
            break;
        }

        let json_str = &decoder[..end_idx];
        decoder = &decoder[end_idx..].trim_start();

        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(json_str) {
            let path = obj["Path"].as_str().unwrap_or("").to_string();
            let version = obj["Version"].as_str().unwrap_or("").to_string();
            let update_version = obj
                .get("Update")
                .and_then(|u| u["Version"].as_str())
                .map(|s| s.to_string());
            let indirect = obj["Indirect"].as_bool().unwrap_or(false);

            // Skip the main module (no version)
            if !version.is_empty() {
                modules.push(GoModule {
                    path,
                    version,
                    update_version,
                    indirect,
                });
            }
        }
    }

    Ok(modules)
}

pub fn get_vulncheck(root: &Path) -> Result<Vec<GoVulnerability>, String> {
    // Check if govulncheck is available
    if which::which("govulncheck").is_err() {
        return Ok(vec![]);
    }

    let output = Command::new("govulncheck")
        .args(["-json", "./..."])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run govulncheck: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut vulns = Vec::new();

    // govulncheck -json outputs newline-delimited JSON messages
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
            // Look for finding messages
            if let Some(finding) = obj.get("finding") {
                let osv = finding["osv"].as_str().unwrap_or("").to_string();
                // Look for the corresponding osv entry
                if !osv.is_empty() && !vulns.iter().any(|v: &GoVulnerability| v.id == osv) {
                    vulns.push(GoVulnerability {
                        id: osv,
                        aliases: vec![],
                        summary: String::new(),
                        module_path: String::new(),
                        found_version: String::new(),
                        fixed_version: String::new(),
                    });
                }
            }
            // OSV data
            if let Some(osv_data) = obj.get("osv") {
                let id = osv_data["id"].as_str().unwrap_or("").to_string();
                let summary = osv_data["summary"].as_str().unwrap_or("").to_string();
                let aliases: Vec<String> = osv_data
                    .get("aliases")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut module_path = String::new();
                let mut found_version = String::new();
                let mut fixed_version = String::new();

                if let Some(affected) = osv_data.get("affected").and_then(|a| a.as_array()) {
                    if let Some(first) = affected.first() {
                        module_path = first["package"]["name"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        if let Some(ranges) = first.get("ranges").and_then(|r| r.as_array()) {
                            for range in ranges {
                                if let Some(events) =
                                    range.get("events").and_then(|e| e.as_array())
                                {
                                    for event in events {
                                        if let Some(introduced) = event["introduced"].as_str() {
                                            found_version = introduced.to_string();
                                        }
                                        if let Some(fixed) = event["fixed"].as_str() {
                                            fixed_version = fixed.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Update or add
                if let Some(existing) = vulns.iter_mut().find(|v| v.id == id) {
                    existing.aliases = aliases;
                    existing.summary = summary;
                    existing.module_path = module_path;
                    existing.found_version = found_version;
                    existing.fixed_version = fixed_version;
                } else if !id.is_empty() {
                    vulns.push(GoVulnerability {
                        id,
                        aliases,
                        summary,
                        module_path,
                        found_version,
                        fixed_version,
                    });
                }
            }
        }
    }

    Ok(vulns)
}

pub fn get_snapshot(root: &Path) -> Result<GoHealthSnapshot, String> {
    let modules = get_modules(root).unwrap_or_default();
    let vulnerabilities = get_vulncheck(root).unwrap_or_default();
    let outdated_count = modules.iter().filter(|m| m.update_version.is_some()).count();
    let vuln_count = vulnerabilities.len();

    Ok(GoHealthSnapshot {
        modules,
        vulnerabilities,
        outdated_count,
        vuln_count,
    })
}
