mod model;

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
