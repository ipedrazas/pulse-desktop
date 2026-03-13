use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMapResult {
    pub framework: String,
    pub endpoints: Vec<ApiEndpoint>,
}

#[tauri::command]
pub async fn discover_api_map(root_path: String) -> Result<ApiMapResult, String> {
    let root = Path::new(&root_path);

    // Try Node/Express first
    if root.join("package.json").exists() {
        if let Some(result) = discover_express_routes(root) {
            return Ok(result);
        }
    }

    // Try Go
    if root.join("go.mod").exists() {
        if let Some(result) = discover_go_routes(root) {
            return Ok(result);
        }
    }

    Ok(ApiMapResult {
        framework: "unknown".to_string(),
        endpoints: vec![],
    })
}

fn discover_express_routes(root: &Path) -> Option<ApiMapResult> {
    // Use ripgrep to find route definitions
    let patterns = [
        // Express: app.get('/path', handler), router.post('/path', handler)
        r#"(?:app|router)\.(get|post|put|delete|patch|options|all)\s*\(\s*['"](/[^'"]*)['"]\s*,"#,
        // Express: app.use('/path', router)
        r#"(?:app|router)\.use\s*\(\s*['"](/[^'"]*)['"]\s*,"#,
    ];

    let mut endpoints = Vec::new();

    for pattern_str in &patterns {
        let re = Regex::new(pattern_str).ok()?;

        let output = std::process::Command::new("rg")
            .args([
                "--line-number",
                "--no-heading",
                "--color=never",
                "-e",
                pattern_str,
                "--type",
                "js",
                "--type",
                "ts",
            ])
            .current_dir(root)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() < 3 {
                continue;
            }

            let file = parts[0].to_string();
            let line_num: u32 = parts[1].parse().unwrap_or(0);
            let content = parts[2];

            if let Some(caps) = re.captures(content) {
                let method = caps
                    .get(1)
                    .map(|m| m.as_str().to_uppercase())
                    .unwrap_or_else(|| "USE".to_string());
                let path = caps
                    .get(2)
                    .or_else(|| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                // Extract handler name heuristically
                let handler = extract_handler_name(content);

                endpoints.push(ApiEndpoint {
                    method,
                    path,
                    handler,
                    file,
                    line: line_num,
                });
            }
        }
    }

    if endpoints.is_empty() {
        return None;
    }

    // Deduplicate
    endpoints.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));
    endpoints.dedup_by(|a, b| a.path == b.path && a.method == b.method);

    Some(ApiMapResult {
        framework: "express".to_string(),
        endpoints,
    })
}

fn discover_go_routes(root: &Path) -> Option<ApiMapResult> {
    // Go patterns: net/http, chi, gin, echo
    let patterns = [
        // net/http: http.HandleFunc("/path", handler)
        r#"(?:http\.HandleFunc|mux\.HandleFunc|r\.HandleFunc)\s*\(\s*"(/[^"]*)""#,
        // chi/gin: r.Get("/path", handler), r.POST("/path", handler)
        r#"\.(?:Get|Post|Put|Delete|Patch|Options|Head|Connect|Trace|Handle|Group)\s*\(\s*"(/[^"]*)""#,
    ];

    let mut endpoints = Vec::new();

    for pattern_str in &patterns {
        let re = Regex::new(pattern_str).ok()?;

        let output = std::process::Command::new("rg")
            .args([
                "--line-number",
                "--no-heading",
                "--color=never",
                "-e",
                pattern_str,
                "--type",
                "go",
            ])
            .current_dir(root)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() < 3 {
                continue;
            }

            let file = parts[0].to_string();
            let line_num: u32 = parts[1].parse().unwrap_or(0);
            let content = parts[2];

            if let Some(caps) = re.captures(content) {
                let path = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();

                // Detect HTTP method from the function call
                let method = detect_go_http_method(content);
                let handler = extract_handler_name(content);

                endpoints.push(ApiEndpoint {
                    method,
                    path,
                    handler,
                    file,
                    line: line_num,
                });
            }
        }
    }

    if endpoints.is_empty() {
        return None;
    }

    endpoints.sort_by(|a, b| a.path.cmp(&b.path).then(a.method.cmp(&b.method)));
    endpoints.dedup_by(|a, b| a.path == b.path && a.method == b.method);

    Some(ApiMapResult {
        framework: "go".to_string(),
        endpoints,
    })
}

fn detect_go_http_method(content: &str) -> String {
    let lower = content.to_lowercase();
    for method in &["get", "post", "put", "delete", "patch", "options", "head"] {
        if lower.contains(&format!(".{}(", method))
            || lower.contains(&format!(".{}(", method.to_uppercase()))
        {
            return method.to_uppercase();
        }
    }
    if lower.contains("handlefunc") || lower.contains("handle(") {
        return "ANY".to_string();
    }
    "ANY".to_string()
}

fn extract_handler_name(content: &str) -> String {
    // Try to find function name after the route path
    let re = Regex::new(r#"['"]\s*,\s*(\w+)"#).unwrap();
    if let Some(caps) = re.captures(content) {
        return caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    }
    // Try arrow function / anonymous
    if content.contains("=>") || content.contains("func(") {
        return "(anonymous)".to_string();
    }
    String::new()
}
