mod config;
mod args;
mod logs;

use crate::config::{SystemConfig};
use crate::args::{parse_args};
use crate::logs::{init_logging};

use tracing::info;

fn main() -> Result<(), lexopt::Error> {
    let args = parse_args()?;
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
    
    info!("TRAHL is initializing...");

    Ok(())
}
