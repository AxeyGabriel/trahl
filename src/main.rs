use std::path::{PathBuf};
use lexopt::prelude::*;

#[derive(Debug)]
struct StartupArgs {
    worker_mode: bool,
    master_mode: bool,
    config_file: PathBuf,
}

fn main() -> Result<(), lexopt::Error> {
    let args = parse_args()?;
    println!("args: {:#?}", args);
    Ok(())
}

fn print_usage() {
    let msg = r#"Usage: thral [-m|--master] [-w|--worker] -c|--conf=file

Options:
    -m, --master    Run in master mode
    -w, --worker    Run in worker mode
    -c, --conf      Configuration file (required)
    -h, --help      Print this help message"#;

    eprintln!("{}", msg);
}

fn parse_args() -> Result<StartupArgs, lexopt::Error> {
    let mut wm = false;
    let mut mm = false;
    let mut cf: Option<PathBuf> = None;

    let mut parser = lexopt::Parser::from_env();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('m') | Long("master") => {
                mm = true;
            }
            Short('w') | Long("worker") => {
                wm = true;
            }
            Short('c') | Long("conf") => {
                let path = parser.value()?;
                cf = Some(PathBuf::from(path));
            }
            Short('h') | Long("help") => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    if !wm && !mm {
        eprintln!("Error: You must specify at least master or worker mode");
        print_usage();
        std::process::exit(1);
    }

    Ok(StartupArgs {
        worker_mode: wm,
        master_mode: mm,
        config_file: cf.ok_or("Missing configuration file")?
    })
}
