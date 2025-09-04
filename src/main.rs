mod config;
mod args;

use crate::config::{SystemConfig};
use crate::args::{parse_args};

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
    
    Ok(())
}
