use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::process::Stdio;
use tauri::{Emitter, State, Window};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ManagedProcess {
    pub id: i64,
    pub project_id: String,
    pub service_name: String,
    pub pid: i32,
    pub command: String,
    pub ports: Option<String>,
    pub started_at: String,
    pub stopped_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ServiceLogEvent {
    service_name: String,
    line: String,
    stream: String,
}

#[tauri::command]
pub async fn start_service(
    pool: State<'_, SqlitePool>,
    window: Window,
    project_id: String,
    service_name: String,
    command: String,
    cwd: String,
    ports: Vec<u16>,
) -> Result<i64, String> {
    // Check for port conflicts
    for port in &ports {
        if is_port_in_use(*port) {
            return Err(format!("Port {} is already in use", port));
        }
    }

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut child = tokio::process::Command::new(parts[0])
        .args(&parts[1..])
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let pid = child.id().unwrap_or(0) as i32;
    let ports_json = serde_json::to_string(&ports).unwrap_or_else(|_| "[]".to_string());

    let process_id: i64 = sqlx::query_scalar(
        "INSERT INTO processes (project_id, service_name, pid, command, ports)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&project_id)
    .bind(&service_name)
    .bind(pid)
    .bind(&command)
    .bind(&ports_json)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let pool_clone = pool.inner().clone();
    let window_clone = window.clone();
    let svc_name = service_name.clone();

    // Stream stdout
    let stdout = child.stdout.take();
    let window_stdout = window_clone.clone();
    let svc_stdout = svc_name.clone();
    tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = window_stdout.emit(
                    "service:log",
                    ServiceLogEvent {
                        service_name: svc_stdout.clone(),
                        line,
                        stream: "stdout".to_string(),
                    },
                );
            }
        }
    });

    // Stream stderr
    let stderr = child.stderr.take();
    let window_stderr = window_clone.clone();
    let svc_stderr = svc_name.clone();
    tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = window_stderr.emit(
                    "service:log",
                    ServiceLogEvent {
                        service_name: svc_stderr.clone(),
                        line,
                        stream: "stderr".to_string(),
                    },
                );
            }
        }
    });

    // Wait for exit in background
    tokio::spawn(async move {
        let _ = child.wait().await;
        let _ = sqlx::query("UPDATE processes SET stopped_at = datetime('now') WHERE id = ?")
            .bind(process_id)
            .execute(&pool_clone)
            .await;
    });

    Ok(process_id)
}

#[tauri::command]
pub async fn stop_service(
    pool: State<'_, SqlitePool>,
    process_id: i64,
) -> Result<(), String> {
    let proc: Option<ManagedProcess> =
        sqlx::query_as("SELECT * FROM processes WHERE id = ? AND stopped_at IS NULL")
            .bind(process_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    if let Some(proc) = proc {
        unsafe {
            libc::kill(proc.pid, libc::SIGTERM);
        }
        sqlx::query("UPDATE processes SET stopped_at = datetime('now') WHERE id = ?")
            .bind(process_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_services(
    pool: State<'_, SqlitePool>,
    project_id: String,
) -> Result<Vec<ManagedProcess>, String> {
    let procs: Vec<ManagedProcess> = sqlx::query_as(
        "SELECT * FROM processes WHERE project_id = ? ORDER BY started_at DESC",
    )
    .bind(&project_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(procs)
}

#[tauri::command]
pub async fn check_port(port: u16) -> Result<bool, String> {
    Ok(is_port_in_use(port))
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

#[tauri::command]
pub async fn check_service_health(url: String) -> Result<bool, String> {
    // Simple TCP check for the health URL
    let url_parsed = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(&url);

    let host_port: Vec<&str> = url_parsed.split('/').next().unwrap_or("").split(':').collect();
    if host_port.len() < 2 {
        return Ok(false);
    }

    let host = host_port[0];
    let port: u16 = host_port[1].parse().map_err(|_| "Invalid port".to_string())?;

    match std::net::TcpStream::connect_timeout(
        &format!("{}:{}", host, port).parse().map_err(|e: std::net::AddrParseError| e.to_string())?,
        std::time::Duration::from_secs(2),
    ) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
