mod config;
mod args;
mod logs;
mod master;
mod worker;
mod lua;

use crate::config::SystemConfig;
use crate::args::parse_args;
use crate::logs::init_logging;
use crate::master::master_thread;
use crate::worker::worker_thread;

use std::io::Error;
use tracing::info;
use signal_hook::flag;
use signal_hook::consts::signal::{SIGINT, SIGTERM, SIGHUP};
use std::sync::{Arc, OnceLock};
use std::sync::atomic::AtomicBool;
use std::thread;

pub static CONFIG: OnceLock<SystemConfig> = OnceLock::new();
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
    CONFIG.set(config).unwrap();
    let config_ref = CONFIG.get().unwrap();


    if args.config_test {
        println!("Configuration test OK");
        std::process::exit(0);
    }


    let _guard = init_logging(&config_ref.log);

    S_TERMINATE.set(Arc::new(AtomicBool::new(false))).unwrap();
    let s_term = S_TERMINATE.get().unwrap();

    S_RELOAD.set(Arc::new(AtomicBool::new(false))).unwrap();
    let s_hup = S_RELOAD.get().unwrap();

    flag::register(SIGINT, s_term.clone())?;
    flag::register(SIGTERM, s_term.clone())?;
    flag::register(SIGHUP, s_hup.clone())?;
    
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
