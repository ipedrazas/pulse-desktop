use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaDependency {
    pub group: String,
    pub artifact: String,
    pub current: String,
    pub latest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaHealthSnapshot {
    pub outdated: Vec<JavaDependency>,
    pub outdated_count: usize,
    pub java_version: Option<String>,
    pub build_tool: String,
}

fn detect_build_tool(root: &Path) -> Option<&'static str> {
    if root.join("pom.xml").exists() {
        Some("maven")
    } else if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
        Some("gradle")
    } else {
        None
    }
}

pub fn get_outdated(root: &Path) -> Result<Vec<JavaDependency>, String> {
    let tool = detect_build_tool(root).ok_or("No Java build tool detected")?;

    match tool {
        "maven" => get_maven_outdated(root),
        "gradle" => get_gradle_outdated(root),
        _ => Ok(vec![]),
    }
}

fn get_maven_outdated(root: &Path) -> Result<Vec<JavaDependency>, String> {
    // mvn versions:display-dependency-updates
    let mvn = if root.join("mvnw").exists() {
        "./mvnw"
    } else {
        "mvn"
    };

    let output = Command::new(mvn)
        .args([
            "versions:display-dependency-updates",
            "-DprocessDependencyManagement=false",
            "-q",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run mvn: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut deps = Vec::new();

    // Parse lines like: "  com.example:artifact .................. 1.0.0 -> 2.0.0"
    let re = regex::Regex::new(
        r"(\S+):(\S+)\s+\.+\s+(\S+)\s+->\s+(\S+)",
    )
    .unwrap();

    for line in stdout.lines() {
        if let Some(caps) = re.captures(line) {
            deps.push(JavaDependency {
                group: caps[1].to_string(),
                artifact: caps[2].to_string(),
                current: caps[3].to_string(),
                latest: caps[4].to_string(),
            });
        }
    }

    Ok(deps)
}

fn get_gradle_outdated(root: &Path) -> Result<Vec<JavaDependency>, String> {
    // Try gradle-versions-plugin output
    let gradle = if root.join("gradlew").exists() {
        "./gradlew"
    } else {
        "gradle"
    };

    let output = Command::new(gradle)
        .args(["dependencyUpdates", "-q"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run gradle: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut deps = Vec::new();

    let re = regex::Regex::new(
        r"(\S+):(\S+)\s+\[(\S+)\s+->\s+(\S+)\]",
    )
    .unwrap();

    for line in stdout.lines() {
        if let Some(caps) = re.captures(line) {
            deps.push(JavaDependency {
                group: caps[1].to_string(),
                artifact: caps[2].to_string(),
                current: caps[3].to_string(),
                latest: caps[4].to_string(),
            });
        }
    }

    Ok(deps)
}

pub fn get_snapshot(root: &Path) -> Result<JavaHealthSnapshot, String> {
    let tool = detect_build_tool(root)
        .unwrap_or("unknown")
        .to_string();

    let java_version = Command::new("java")
        .args(["-version"])
        .output()
        .ok()
        .map(|o| {
            // java -version outputs to stderr
            String::from_utf8_lossy(&o.stderr)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .filter(|s| !s.is_empty());

    let outdated = get_outdated(root).unwrap_or_default();
    let outdated_count = outdated.len();

    Ok(JavaHealthSnapshot {
        outdated,
        outdated_count,
        java_version,
        build_tool: tool,
    })
}
