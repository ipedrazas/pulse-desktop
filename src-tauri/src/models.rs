use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// --- Database models ---

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub remote_url: Option<String>,
    pub created_at: String,
    pub last_opened: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Run {
    pub id: i64,
    pub project_id: String,
    pub kind: String,
    pub macro_id: Option<String>,
    pub status: String,
    pub command: String,
    pub cwd: String,
    pub env_keys: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub pid: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RunLog {
    pub id: i64,
    pub run_id: i64,
    pub stream: String,
    pub chunk: String,
    pub ts: String,
}

// --- .pulse.yaml models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseConfig {
    pub version: Option<i32>,
    pub context: Option<ContextConfig>,
    pub services: Option<Vec<ServiceConfig>>,
    pub macros: Option<Vec<MacroConfig>>,
    pub watchers: Option<Vec<WatcherConfig>>,
    pub connectors: Option<Vec<ConnectorConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub files: Option<Vec<String>>,
    pub copy_bundle: Option<CopyBundleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyBundleConfig {
    pub max_bytes: Option<usize>,
    pub include_git: Option<GitInclude>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInclude {
    pub branch: Option<bool>,
    pub sha: Option<bool>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub cwd: Option<String>,
    #[serde(rename = "type")]
    pub service_type: Option<String>,
    pub dev: Option<DevConfig>,
    pub health: Option<HealthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    pub command: String,
    pub ports: Option<Vec<u16>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroConfig {
    pub id: String,
    pub title: String,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroStep {
    pub run: String,
    pub cwd: Option<String>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub id: String,
    pub title: String,
    pub enabled: Option<bool>,
    pub glob: Option<String>,
    pub debounce_ms: Option<u64>,
    #[serde(rename = "macro")]
    pub macro_ref: Option<String>,
    pub concurrency: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    pub title: String,
    pub command: Option<String>,
    pub url: Option<String>,
    pub variables: Option<serde_json::Value>,
}

// --- Git info ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub sha: String,
    pub dirty: bool,
}

// --- Project with extra info for UI ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    #[serde(flatten)]
    pub project: Project,
    pub project_type: Option<String>,
    pub languages: Option<Vec<String>>,
    pub language_dirs: Option<BTreeMap<String, Vec<String>>>,
    pub git: Option<GitInfo>,
    pub has_pulse_yaml: bool,
    pub has_a2_yaml: bool,
}
