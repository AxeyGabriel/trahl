use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct RowWorker {
    pub id: i64,
    pub identifier: String,
    pub last_conn_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RowScript {
    pub id: i64,
    pub name: String,
    pub hash: String,
    pub script: String,
    pub source: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RowLibrary {
    pub id: i64,
    pub name: String,
    pub source: String,
    pub destination: String,
    pub enabled: i64,
    pub path: String,
    pub script_id: Option<i64>,
    pub last_scanned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RowVariable {
    pub id: i64,
    pub key: String,
    pub value: Option<String>,
    pub library_id: Option<i64>
}

#[derive(Debug, Clone, FromRow)]
pub struct RowFileEntry {
    pub id: i64,
    pub library_id: i64,
    pub job_id: Option<i64>,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub hash: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct RowJob {
    pub id: i64,
    pub file_id: i64,
    pub worker_id: Option<i64>,
    pub status: String,
    pub log_path: Option<String>,
    pub output_file: Option<String>,
    pub output_size: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}
