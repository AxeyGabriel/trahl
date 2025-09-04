use std::path::{PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging(log_file: Option<PathBuf>) -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_new("info")
        .expect("Error setting log level");

    match &log_file {
        Some(path) => {
            let dir = path.parent().expect("Could not extract log path");
            let file = path.file_name().expect("Could not extract log file name");
            let appender = tracing_appender::rolling::never(dir, file);
            let (nb, _guard) = tracing_appender::non_blocking(appender);

            let subscriber = fmt()
                .with_env_filter(env_filter)
                .with_thread_names(true)
                .with_ansi(false)
                .with_writer(nb)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set local subscriber");

                return Some(_guard);
        },
        None => {
            let subscriber = fmt()
                .with_env_filter(env_filter)
                .with_thread_names(true)
                .with_ansi(true)
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set local subscriber");

                return None;
        }
    };
}
