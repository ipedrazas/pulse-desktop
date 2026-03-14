use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;

pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let db_path = db_path();

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

fn db_path() -> PathBuf {
    let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("dev.andcake.pulse").join("pulse.db")
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS projects (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            root_path   TEXT NOT NULL UNIQUE,
            remote_url  TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            last_opened TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_files (
            project_id  TEXT NOT NULL REFERENCES projects(id),
            path        TEXT NOT NULL,
            kind        TEXT NOT NULL,
            last_hash   TEXT,
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (project_id, path)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS runs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            kind        TEXT NOT NULL,
            macro_id    TEXT,
            status      TEXT NOT NULL DEFAULT 'running',
            command     TEXT NOT NULL,
            cwd         TEXT NOT NULL,
            env_keys    TEXT,
            exit_code   INTEGER,
            started_at  TEXT NOT NULL DEFAULT (datetime('now')),
            finished_at TEXT,
            pid         INTEGER
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS run_logs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id      INTEGER NOT NULL REFERENCES runs(id),
            stream      TEXT NOT NULL DEFAULT 'stdout',
            chunk       TEXT NOT NULL,
            ts          TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            kind        TEXT NOT NULL,
            commit_sha  TEXT,
            payload     TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agent_sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            title       TEXT,
            tool        TEXT,
            task_summary TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS processes (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            service_name TEXT NOT NULL,
            pid         INTEGER NOT NULL,
            command     TEXT NOT NULL,
            ports       TEXT,
            started_at  TEXT NOT NULL DEFAULT (datetime('now')),
            stopped_at  TEXT
        )",
    )
    .execute(pool)
    .await?;

    // Phase 4 tables

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workspace_projects (
            workspace_id TEXT NOT NULL REFERENCES workspaces(id),
            project_id   TEXT NOT NULL REFERENCES projects(id),
            added_at     TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (workspace_id, project_id)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS semantic_chunks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            file_path   TEXT NOT NULL,
            chunk_type  TEXT NOT NULL,
            name        TEXT,
            content     TEXT NOT NULL,
            language    TEXT,
            start_line  INTEGER,
            end_line    INTEGER,
            commit_sha  TEXT,
            keywords    TEXT,
            indexed_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_semantic_chunks_project
         ON semantic_chunks(project_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_semantic_chunks_keywords
         ON semantic_chunks(keywords)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS plugins (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            description TEXT,
            version     TEXT,
            entry_point TEXT NOT NULL,
            plugin_type TEXT NOT NULL,
            enabled     INTEGER NOT NULL DEFAULT 1,
            config      TEXT,
            installed_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}
