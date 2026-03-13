use crate::models::{MacroStep, Run, RunLog};
use sqlx::SqlitePool;
use std::process::Stdio;
use tauri::{Emitter, State, Window};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone, serde::Serialize)]
struct RunLogEvent {
    run_id: i64,
    stream: String,
    line: String,
}

#[derive(Clone, serde::Serialize)]
struct RunStatusEvent {
    run_id: i64,
    status: String,
    exit_code: Option<i32>,
}

#[tauri::command]
pub async fn execute_macro_step(
    pool: State<'_, SqlitePool>,
    window: Window,
    project_id: String,
    macro_id: String,
    step: MacroStep,
    cwd: String,
) -> Result<i64, String> {
    // Parse command into program and args
    let parts: Vec<&str> = step.run.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let program = parts[0];
    let args = &parts[1..];

    let step_cwd = if let Some(ref step_dir) = step.cwd {
        let base = std::path::Path::new(&cwd);
        base.join(step_dir).to_string_lossy().to_string()
    } else {
        cwd.clone()
    };

    // Insert run record
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO runs (project_id, kind, macro_id, status, command, cwd)
         VALUES (?, 'macro', ?, 'running', ?, ?)
         RETURNING id",
    )
    .bind(&project_id)
    .bind(&macro_id)
    .bind(&step.run)
    .bind(&step_cwd)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Spawn the process
    let mut child = tokio::process::Command::new(program)
        .args(args)
        .current_dir(&step_cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    // Update PID
    if let Some(pid) = child.id() {
        let _ = sqlx::query("UPDATE runs SET pid = ? WHERE id = ?")
            .bind(pid as i32)
            .bind(run_id)
            .execute(pool.inner())
            .await;
    }

    let pool_clone = pool.inner().clone();
    let window_clone = window.clone();

    // Stream stdout
    let stdout = child.stdout.take();
    let pool_stdout = pool_clone.clone();
    let window_stdout = window_clone.clone();
    let run_id_stdout = run_id;
    let stdout_handle = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = sqlx::query(
                    "INSERT INTO run_logs (run_id, stream, chunk) VALUES (?, 'stdout', ?)",
                )
                .bind(run_id_stdout)
                .bind(&line)
                .execute(&pool_stdout)
                .await;

                let _ = window_stdout.emit(
                    "run:log",
                    RunLogEvent {
                        run_id: run_id_stdout,
                        stream: "stdout".to_string(),
                        line,
                    },
                );
            }
        }
    });

    // Stream stderr
    let stderr = child.stderr.take();
    let pool_stderr = pool_clone.clone();
    let window_stderr = window_clone.clone();
    let run_id_stderr = run_id;
    let stderr_handle = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = sqlx::query(
                    "INSERT INTO run_logs (run_id, stream, chunk) VALUES (?, 'stderr', ?)",
                )
                .bind(run_id_stderr)
                .bind(&line)
                .execute(&pool_stderr)
                .await;

                let _ = window_stderr.emit(
                    "run:log",
                    RunLogEvent {
                        run_id: run_id_stderr,
                        stream: "stderr".to_string(),
                        line,
                    },
                );
            }
        }
    });

    // Wait for process to complete
    let pool_final = pool_clone;
    let window_final = window_clone;
    tokio::spawn(async move {
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        let status = child.wait().await;
        let (run_status, exit_code) = match status {
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                if s.success() {
                    ("success".to_string(), Some(code))
                } else {
                    ("failure".to_string(), Some(code))
                }
            }
            Err(_) => ("failure".to_string(), None),
        };

        let _ = sqlx::query(
            "UPDATE runs SET status = ?, exit_code = ?, finished_at = datetime('now') WHERE id = ?",
        )
        .bind(&run_status)
        .bind(exit_code)
        .bind(run_id)
        .execute(&pool_final)
        .await;

        let _ = window_final.emit(
            "run:status",
            RunStatusEvent {
                run_id,
                status: run_status,
                exit_code,
            },
        );
    });

    Ok(run_id)
}

#[tauri::command]
pub async fn list_runs(
    pool: State<'_, SqlitePool>,
    project_id: String,
) -> Result<Vec<Run>, String> {
    let runs: Vec<Run> =
        sqlx::query_as("SELECT * FROM runs WHERE project_id = ? ORDER BY started_at DESC LIMIT 50")
            .bind(&project_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    Ok(runs)
}

#[tauri::command]
pub async fn get_run_logs(
    pool: State<'_, SqlitePool>,
    run_id: i64,
) -> Result<Vec<RunLog>, String> {
    let logs: Vec<RunLog> =
        sqlx::query_as("SELECT * FROM run_logs WHERE run_id = ? ORDER BY id ASC")
            .bind(run_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    Ok(logs)
}

#[tauri::command]
pub async fn cancel_run(run_id: i64, pool: State<'_, SqlitePool>) -> Result<(), String> {
    // Get PID from database
    let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM runs WHERE id = ? AND status = 'running'")
        .bind(run_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(Some(pid)) = pid.map(Some) {
        // Send SIGTERM
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }

        sqlx::query("UPDATE runs SET status = 'cancelled', finished_at = datetime('now') WHERE id = ?")
            .bind(run_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
