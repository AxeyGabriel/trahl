use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};
use crate::config::LogConfig;

fn init_log_to_file(env_filter: EnvFilter, log_config: &LogConfig) -> Option<WorkerGuard> {
    let path = log_config.file.as_ref().unwrap();
    let dir = path.parent().expect("Could not extract log path");
    let file = path.file_name().expect("Could not extract log file name");
    let appender = tracing_appender::rolling::never(dir, file);
    let (nb, _guard) = tracing_appender::non_blocking(appender);

    let subscriber = fmt()
        .with_env_filter(env_filter)
        .with_thread_names(true)
        .with_ansi(false)
        .with_writer(nb)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set local subscriber");

        return Some(_guard);
}

fn init_log_to_stdout(env_filter: EnvFilter, _log_config: &LogConfig) -> Option<WorkerGuard> {
    let subscriber = fmt()
        .with_env_filter(env_filter)
        .with_thread_names(false)
        .with_ansi(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set local subscriber");

        return None;
}

pub fn init_logging(log_config: &LogConfig) -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_new(&log_config.level)
        .expect("Error setting log level");

    let ret: Option<WorkerGuard>;

    match &log_config.file {
        Some(file) if file.to_str().map_or(false, |f| f == "/dev/stdout") => {
            ret = init_log_to_stdout(env_filter, &log_config);
        },
        Some(_) => {
            ret = init_log_to_file(env_filter, &log_config);
        },
        None => {
            ret = init_log_to_stdout(env_filter, &log_config);
        }
    };

    ret
}
