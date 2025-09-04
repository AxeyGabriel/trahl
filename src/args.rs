use std::{ffi::OsString, path::PathBuf};
use lexopt::prelude::*;
use indoc::indoc;

#[derive(Debug, PartialEq)]
pub struct StartupArgs {
    pub worker_mode: bool,
    pub master_mode: bool,
    pub config_test: bool,
    pub config_file: PathBuf,
}

#[derive(Debug)]
struct CustomError(String);

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CustomError {}

pub fn parse_args() -> Result<StartupArgs, lexopt::Error> {
    parse_args_from(std::env::args_os())
}

pub fn parse_args_from<I>(args: I) -> Result<StartupArgs, lexopt::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let mut wm = false;
    let mut mm = false;
    let mut ct = false;
    let mut cf: Option<PathBuf> = None;

    let mut parser = lexopt::Parser::from_iter(args);

    while let Some(arg) = parser.next()? {
        match arg {
            Short('m') | Long("master") => {
                mm = true;
            }
            Short('w') | Long("worker") => {
                wm = true;
            }
            Short('t') => {
                ct = true;
            }
            Short('c') | Long("config") => {
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
        let err = "You must specify at lest one working mode";
        return Err(lexopt::Error::Custom(Box::new(CustomError(err.to_string()))));
    }

    Ok(StartupArgs {
        worker_mode: wm,
        master_mode: mm,
        config_test: ct,
        config_file: cf.ok_or("Missing configuration file")?
    })
}

fn print_usage() {
    let msg = indoc!{r#"
        Usage: trahl [-m|--master] [-w|--worker] [-t] -c|--conf=file

        Options:
            -m, --master    Run in master mode
            -w, --worker    Run in worker mode
            -t              Test configuration and exit
            -c, --config    Configuration file (required)
            -h, --help      Print this help message"#
    };

    eprintln!("{}", msg);
}

#[cfg(test)]
mod tests {
    use super::{StartupArgs, parse_args_from};
    use std::ffi::OsString;

    fn parse_args_from_string(s: &str) -> Result<StartupArgs, lexopt::Error> 
    {
        let mut args: Vec<OsString> = vec![OsString::from("dummy")]; /* argv[0] */
        args.extend(s.split_whitespace().map(OsString::from));

        parse_args_from(args) 
    }

    #[test]
    fn test_no_args() {
        let parsed = parse_args_from_string("");
        match parsed {
            Err(_) => {},
            _ => {
                panic!("Empty arguments list should fail");
            }
        }
    }
    
    #[test]
    fn test_args_no_conf() {
        let args = "-m";
        let parsed = parse_args_from_string(args);
        match parsed {
            Err(_) => {},
            _ => {
                panic!("No configuration file should fail");
            }
        }
    }
    
    #[test]
    fn test_args_ok_short() {
        let args = "-m -w -c dummy.toml";
        let parsed = parse_args_from_string(args).unwrap();

        assert_eq!(parsed, StartupArgs {
            master_mode: true,
            worker_mode: true,
            config_file: "dummy.toml".into(),
            config_test: false,
        });
    }
    
    #[test]
    fn test_args_ok_long() {
        let args = "--master --worker --config dummy.toml";
        let parsed = parse_args_from_string(args).unwrap();

        assert_eq!(parsed, StartupArgs {
            master_mode: true,
            worker_mode: true,
            config_file: "dummy.toml".into(),
            config_test: false,
        });
    }
    
    #[test]
    fn test_args_ok_long_short_equal() {
        let args = "-m --worker --config=dummy.toml";
        let parsed = parse_args_from_string(args).unwrap();

        assert_eq!(parsed, StartupArgs {
            master_mode: true,
            worker_mode: true,
            config_file: "dummy.toml".into(),
            config_test: false,
        });
    }
}
