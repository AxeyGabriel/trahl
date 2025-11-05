pub mod model;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::info;
use sqlx::{
    migrate::Migrator,
    migrate::MigrateDatabase,
    Pool,
    Sqlite,
    SqlitePool,
};
use chrono::Utc;
use xxhash_rust::xxh3::xxh3_64;
use crate::config::JobConfig;

pub static DB: OnceLock<Pool<Sqlite>> = OnceLock::new();

pub async fn init_db(path: PathBuf) {
    let connstr = format!("sqlite://{}", path.to_string_lossy());
    if !Sqlite::database_exists(&connstr).await.unwrap_or(false) {
        info!("Creating database {}", connstr);
        if let Err(e) = Sqlite::create_database(&connstr).await {
            panic!("Could not create database: {}", e);
        }
    }

    let db = match SqlitePool::connect(&connstr).await {
        Ok(v) => v,
        Err(e) => {
            panic!("Could not connect to database: {}", e);
        }
    };

    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let migrations_dir = Path::new(&crate_dir).join("./migrations");
    let migrations_result = Migrator::new(migrations_dir)
        .await
        .unwrap()
        .run(&db)
        .await;

    match migrations_result {
        Ok(_) => {
            info!("Database migration success");
        }
        Err(e) => {
            panic!("Error migrating database: {}", e);
        }
    }

    DB.set(db).expect("Failed to set global DB pool");
}

async fn load_lua_script(path: &PathBuf) -> (String, String) {
    let content = tokio::fs::read_to_string(path)
        .await
        .unwrap_or_else(|_| panic!("Failed to read Lua script: {}", path.display()));

    // Compute fast 64-bit xxHash and convert to hex
    let hash = format!("{:016x}", xxh3_64(content.as_bytes()));

    (content, hash)
}

pub async fn merge_libs_config(configs: &[JobConfig]) {
    let pool = DB.get().unwrap();
    let config_names: Vec<&str> = configs.iter().map(|c| c.name.as_str()).collect();

    for cfg in configs {
        let (script_contents, script_hash) = load_lua_script(&cfg.lua_script).await;
        let script_source = format!("file://{}", cfg.lua_script.display());
        let script_name = cfg
            .lua_script
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Check if script exists with same hash
        let existing_script = sqlx::query!(
            r#"
            SELECT id, hash FROM script
            WHERE name = ? AND source = ?
            "#,
            script_name,
            script_source
        )
        .fetch_optional(pool)
        .await
        .expect("Failed to query existing script");

        let script_id: i64 = match existing_script {
            Some(row) if row.hash == script_hash => {
                // Script unchanged — skip update
                row.id.expect("ID is never null")
            }
            Some(row) => {
                // Script changed, update
                sqlx::query!(
                    r#"
                    UPDATE script
                    SET script = ?, hash = ?, updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    "#,
                    script_contents,
                    script_hash,
                    row.id
                )
                .execute(pool)
                .await
                .expect("Failed to update script");
                row.id.expect("ID is never null")
            }
            None => {
                // Script doesnt exist, insert new one
                sqlx::query!(
                    r#"
                    INSERT INTO script (name, hash, script, source)
                    VALUES (?, ?, ?, ?)
                    "#,
                    script_name,
                    script_hash,
                    script_contents,
                    script_source,
                )
                .execute(pool)
                .await
                .expect("Failed to insert new script")
                .last_insert_rowid()
            }
        };

        // Upsert library
        let enabled_int = if cfg.enabled { 1 } else { 0 };
        let now = Utc::now();
        let dest_str = cfg.destination_path.to_string_lossy().to_string();
        let src_str = cfg.source_path.to_string_lossy().to_string();

        let updated = sqlx::query!(
            r#"
            UPDATE library
            SET path = ?, destination = ?, enabled = ?, script_id = ?, last_scanned_at = ?
            WHERE name = ? AND source = 'conf'
            "#,
            src_str,
            dest_str,
            enabled_int,
            script_id,
            now,
            cfg.name
        )
        .execute(pool)
        .await
        .expect("Failed to update library");

        let library_id: i64 = if updated.rows_affected() == 0 {
            sqlx::query!(
                r#"
                INSERT INTO library (name, source, destination, enabled, path, script_id)
                VALUES (?, 'conf', ?, ?, ?, ?)
                "#,
                cfg.name,
                dest_str,
                enabled_int,
                src_str,
                script_id
            )
            .execute(pool)
            .await
            .expect("Failed to insert library")
            .last_insert_rowid()
        } else {
            sqlx::query!(
                "SELECT id FROM library WHERE name = ? AND source = 'conf'",
                cfg.name
            )
            .fetch_one(pool)
            .await
            .expect("Failed to fetch library ID")
            .id.expect("ID is never null")
        };

        // Replace variables
        sqlx::query!("DELETE FROM variables WHERE library_id = ?", library_id)
            .execute(pool)
            .await
            .expect("Failed to clear old variables");

        for (key, value) in &cfg.variables {
            sqlx::query!(
                r#"
                INSERT INTO variables (key, value, library_id)
                VALUES (?, ?, ?)
                "#,
                key,
                value,
                library_id
            )
            .execute(pool)
            .await
            .expect("Failed to insert variable");
        }

        info!("Synced library '{}' (script: '{}')", cfg.name, script_name);
    }

    // Disable libraries missing from config
    let json_names = serde_json::to_string(&config_names).unwrap();
    let disabled = sqlx::query!(
        r#"
        UPDATE library
        SET enabled = 0
        WHERE source = 'conf'
          AND name NOT IN (
            SELECT value FROM json_each(?)
          )
        "#,
        json_names
    )
    .execute(pool)
    .await
    .expect("Failed to disable missing libraries");

    if disabled.rows_affected() > 0 {
        info!("Disabled {} libraries not in config.", disabled.rows_affected());
    }
}

pub async fn upsert_worker(identifier: &str) {
    let pool = DB.get().unwrap();
    let now = Utc::now();

    let updated = sqlx::query!(
        r#"
        UPDATE workers
        SET last_conn_at = ?
        WHERE identifier = ?
        "#,
        now,
        identifier
    ).execute(pool)
    .await
    .expect("Failed to update table workers");

    if updated.rows_affected() == 0 {
        sqlx::query!(
            r#"
            INSERT INTO workers (identifier, last_conn_at)
            VALUES (?, ?)
            "#,
            identifier, now
        )
        .execute(pool)
        .await
        .expect("Failed to insert into table workers");
    }
}
