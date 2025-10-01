mod config;
mod args;
mod logs;
mod master;
mod worker;
mod lua;
mod extcmd;
mod rpc;
mod utils;

use crate::config::SystemConfig;
use crate::args::parse_args;
use crate::logs::init_logging;
use crate::master::master_thread;
use crate::worker::worker_thread;

use std::io::Error;
use tracing::{info, error};
use signal_hook::iterator::Signals;
use signal_hook::consts::signal::{SIGINT, SIGTERM, SIGHUP};
use std::sync::{Arc, OnceLock, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;

pub static CONFIG: OnceLock<Arc<RwLock<SystemConfig>>> = OnceLock::new();
pub static S_TERMINATE: OnceLock<Arc<AtomicBool>> = OnceLock::new();
pub static S_RELOAD: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn main() -> Result<(), Error> {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments: {}", e);
            std::process::exit(1);
        }
    };

    let config = match SystemConfig::parse(&args.config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config \"{}\": {}", &args.config_file.display(), e);
            std::process::exit(1);
        }
    };
    CONFIG.set(Arc::new(RwLock::new(config))).unwrap();
    let config_ref = CONFIG.get().unwrap().clone();


    if args.config_test {
        println!("Configuration test OK");
        println!("{:#?}", CONFIG.get().unwrap().read().unwrap());
        std::process::exit(0);
    }

    let _guard = init_logging(&config_ref.read().unwrap().log);

    S_TERMINATE.set(Arc::new(AtomicBool::new(false))).unwrap();
    S_RELOAD.set(Arc::new(AtomicBool::new(false))).unwrap();

    let config_clone = CONFIG.get().unwrap().clone();
    let mut signals = Signals::new(&[SIGHUP, SIGINT, SIGTERM])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGINT || sig == SIGTERM {
                if let Some(flag) = S_TERMINATE.get() {
                    flag.store(true, Ordering::Relaxed);
                }
            } else if sig == SIGHUP {
                {
                    let mut cfg = config_clone.write().unwrap();
                    let config = match SystemConfig::parse(&args.config_file) {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            error!("Failed to load config \"{}\": {}", &args.config_file.display(), e);
                            continue;
                        }
                    };
                    *cfg = config;
                    info!("Configuration reloaded");
                }

                if let Some(flag) = S_RELOAD.get() {
                    flag.store(true, Ordering::Relaxed);
                }
            }
        }
    });

    info!("TRAHL is setting up");

    let mut t_handles: [Option<thread::JoinHandle<()>>; 2] = [None, None];

    if args.master_mode {
        let h_master = thread::Builder::new()
            .name("master".into())
            .spawn(master_thread)
            .unwrap();
        t_handles[0] = Some(h_master);
    }

    if args.worker_mode {
        let h_worker = thread::Builder::new()
            .name("worker".into())
            .spawn(worker_thread)
            .unwrap();
        t_handles[1] = Some(h_worker);
    }

    for handle in t_handles.iter_mut() {
        if let Some(handle) = handle.take() {
            let _ = handle.join().unwrap();
        }
    }

    info!("TRAHL is finished");

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;
    use std::str::FromStr;
    use tracing_subscriber::{fmt, EnvFilter};

    pub fn init_tracing() {
        static TRACING: OnceLock<()> = OnceLock::new();
        TRACING.get_or_init(|| {
            let filter = EnvFilter::from_str("debug").unwrap();
            let subscriber = fmt()
                .with_env_filter(filter)
                .finish();
            tracing::subscriber::set_global_default(subscriber).unwrap();
        });
    }
}
