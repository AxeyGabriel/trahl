use std::{
    collections::HashSet,
    sync::Arc,
    path::{
        PathBuf,
        Path,
    },
    time::Instant,
};
use chrono::Utc;
use sqlx::{
    Pool,
    Sqlite,
};
use tokio::{
    sync::{
        Mutex,
        mpsc::Receiver
    },
    fs,
    task,
};
use futures::{
    future::{
        AbortHandle,
        Abortable,
    },
    stream::{
        FuturesUnordered,
        StreamExt,
    }
};
use tracing::{
    trace,
    info,
    warn,
    error,
    debug,
    instrument,
    Span,
};
use anyhow::Result;

use crate::utils;

use super::db::model::{
    Library,
};
use super::MasterCtx;
use super::db::DB;

pub struct Librarian {
    rx: Receiver<i64>,
    active_libs: Arc<Mutex<HashSet<i64>>>
}

impl Librarian {
    pub fn new(rx: Receiver<i64>) -> Self {
        Self {
            rx,
            active_libs: Arc::new(Mutex::new(HashSet::new()))
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let pool = DB.get().unwrap();
        
        let active_libs = Arc::clone(&self.active_libs);
        let mut futures = FuturesUnordered::new();
        let mut abort_handles = Vec::new();

        loop {
            tokio::select! {
                Some(lib_id) = self.rx.recv() => {
                    {
                        let mut active = active_libs.lock().await;
                        if active.contains(&lib_id) {
                            warn!("Library id={} is already scanning, skipping.", lib_id);
                            continue;
                        }

                        active.insert(lib_id);
                    }

                    let active_libs = Arc::clone(&active_libs);
                    let pool = pool.clone();
                    let (handle, reg) = AbortHandle::new_pair();
                    abort_handles.push(handle);

                    futures.push(Abortable::new(async move {
                        if let Err(e) = task_full_scan_library(&pool, lib_id).await {
                            error!("Error scanning library id={}: {}", lib_id, e);
                        }

                        let mut active = active_libs.lock().await;
                        active.remove(&lib_id);
                        lib_id
                    }, reg));

                }

                Some(lib_scan_result) = futures.next() => {
                    if let Err(e) = lib_scan_result {
                        info!("Library scan aborted: {}", e);
                    }
                }
                
                _ = ch_term.changed() => {
                    if *ch_term.borrow() {
                        for handle in abort_handles {
                            handle.abort();
                        }
                        break;
                    }
                }
            }
        }
    }
}

#[instrument(
    name = "full_scan_libray",
    skip(pool),
    fields(elapsed_seconds, scanned_files, scan_rate)
)]
async fn task_full_scan_library(pool: &Pool<Sqlite>, lib_id: i64) -> Result<()> {
        if let Some(library) = sqlx::query_as!(
            Library,
            r#"
            SELECT * FROM library
            WHERE id = ?
            AND enabled = 1
            "#,
            lib_id
        )
        .fetch_optional(pool)
        .await? {
            info!("Starting scan for library name={}", library.name);

            let start_time = Instant::now();
            let num_files = scan_folder(pool, &library, None).await?;
            let duration = start_time.elapsed();
            let seconds = duration.as_secs_f64();
            let rate = if seconds > 0.0 {
                num_files as f64 / seconds
            } else {
                0.0
            };

            let now = Utc::now();

            sqlx::query!(
                r#"
                UPDATE library
                SET last_scanned_at = ?
                WHERE id = ?
                "#,
                now,
                library.id
            )
            .execute(pool)
            .await?;

            let span = Span::current();
            span.record("elapsed_seconds", seconds);
            span.record("scanned_files", num_files);
            span.record("scan_rate", rate);
            
            info!("Finished");
            
        } else {
            error!("Cannot find library");
        }

    Ok(())
}

async fn scan_folder(pool: &Pool<Sqlite>, library: &Library, path_override: Option<&Path>) -> Result<u64> {
    let library_path: PathBuf = path_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&library.path));

    let mut entries = fs::read_dir(library_path).await?;
    let mut num_files = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            num_files += Box::pin(scan_folder(pool, &library, Some(&path))).await?;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        num_files += 1;

        let file_path = path.strip_prefix(&library.path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT file_entry.id
            FROM file_entry
            LEFT JOIN Job ON job.output_file = file_entry.file_path
            WHERE file_entry.file_path = ?
            OR Job.output_file = ?
            LIMIT 1
            "#
        )
        .bind(&file_path)
        .bind(&file_path)
        .fetch_optional(pool)
        .await?
        .is_some();

        if exists {
            // Ignore already known file
            trace!("File={} is already known, skipping", path.to_string_lossy().to_string());
            continue;
        }

        let metadata = match fs::metadata(&path).await {
            Ok(m) => m,
            Err(e) => return Err(e.into()),
        };

        let file_size = metadata.len() as i64;
        
        let path_cloned = path.clone();
        let hash = task::spawn_blocking(move || utils::chunked_hash(path_cloned))
        .await
        .unwrap()?;

        sqlx::query!(
            r#"
            INSERT INTO file_entry (library_id, file_path, file_size, hash)
            VALUES (?, ?, ?, ?)
            "#,
            library.id,
            file_path,
            file_size,
            hash
        )
        .execute(pool)
        .await?;

        debug!("Discovered file for library id={}: path={} size={}, hash={}",
            library.id, file_path, file_size, hash
        );
    }

    Ok(num_files) 
}
