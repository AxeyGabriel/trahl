mod config;
mod args;
mod logs;

use crate::config::{SystemConfig};
use crate::args::{parse_args};
use crate::logs::{init_logging};

use std::io::Error;
use tracing::info;
use signal_hook::flag;
use signal_hook::consts::signal::*;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

pub static S_TERMINATE: OnceLock<Arc<AtomicBool>> = OnceLock::new();

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

    if args.config_test {
        println!("Configuration test OK");
        std::process::exit(0);
    }

    let _guard = init_logging(config.log_file);

    S_TERMINATE.set(Arc::new(AtomicBool::new(false))).unwrap();
    let s_term = S_TERMINATE.get().unwrap();
    flag::register(SIGINT, s_term.clone())?;
    flag::register(SIGTERM, s_term.clone())?;
    
    info!("TRAHL is initializing...");
    
    loop {
        let terminate = S_TERMINATE.get().unwrap();
        if terminate.load(Ordering::Relaxed) {
            info!("term");
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    

    Ok(())
}
