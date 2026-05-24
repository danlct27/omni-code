use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct LogEntry {
    pub timestamp: String,
    pub source: String,
    pub model: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub latency_ms: i64,
    pub status: String,
}

#[derive(Clone)]
pub struct Logger {
    tx: mpsc::Sender<LogEntry>,
}

impl Logger {
    pub fn new() -> Self {
        let db_path = db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path).expect("failed to open logs.db");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=150;
             CREATE TABLE IF NOT EXISTS requests (
                 id INTEGER PRIMARY KEY,
                 timestamp TEXT,
                 source TEXT,
                 model TEXT,
                 tokens_in INTEGER,
                 tokens_out INTEGER,
                 latency_ms INTEGER,
                 status TEXT
             );",
        )
        .expect("failed to create table");

        // P0-2: Bounded channel (1024) to prevent OOM
        let (tx, mut rx) = mpsc::channel::<LogEntry>(1024);
        let conn = Arc::new(std::sync::Mutex::new(conn));

        tokio::spawn(async move {
            while let Some(entry) = rx.recv().await {
                let conn = conn.clone();
                // P0-3: spawn_blocking for synchronous SQLite writes
                tokio::task::spawn_blocking(move || {
                    let conn = conn.lock().unwrap();
                    conn.execute(
                        "INSERT INTO requests (timestamp, source, model, tokens_in, tokens_out, latency_ms, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        rusqlite::params![entry.timestamp, entry.source, entry.model, entry.tokens_in, entry.tokens_out, entry.latency_ms, entry.status],
                    ).ok();
                }).await.ok();
            }
        });

        Self { tx }
    }

    pub fn log_request(&self, entry: LogEntry) {
        // P0-2: try_send — drop entry if channel full, don't block hot path
        if self.tx.try_send(entry).is_err() {
            tracing::warn!("Log channel full, dropping entry");
        }
    }
}

fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".omni-code/logs.db")
}
