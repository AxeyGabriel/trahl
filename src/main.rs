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

#[cfg(test)]
mod tests {
    use tempfile::{NamedTempFile};
    use std::io::Write;
    use indoc::indoc;

    #[test]
    fn test_args_none() {
        assert_cmd::Command::cargo_bin("trahl")
            .unwrap()
            .assert()
            .failure();
    }
    
    #[test]
    fn test_args_no_conf() {
        assert_cmd::Command::cargo_bin("trahl")
            .unwrap()
            .arg("--master")
            .assert()
            .failure();
    }
    
    #[test]
    fn test_args_ok() {
        let mut conf_file = NamedTempFile::new()
            .expect("Failed to create temporary file");

        let conf_content = indoc!{r#"
            log_level="info"
            log_file="trahl.log"
            user="trahl"
            group="media"
            [master]
            orch_bind_addr="0.0.0.0:1849"
            web_bind_addr="0.0.0.0:1850"
            [worker]
            identifier="worker"
            orch_addr="127.0.0.1:1849"
            cache_dir="/tmp/trahl-cache"
            "#};

        write!(conf_file, "{}", conf_content)
            .expect("Failed to write to temporary file");

        assert_cmd::Command::cargo_bin("trahl")
            .unwrap()
            .arg("-t")
            .arg("--master")
            .arg(format!("--conf={}", conf_file.path().to_string_lossy()))
            .assert()
            .success();
    }
}
