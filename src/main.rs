use std::path::{Path, PathBuf};
use std::fs;
use std::net::SocketAddr;
use lexopt::prelude::*;
use serde::Deserialize;
use toml;

#[derive(Debug)]
struct StartupArgs {
    worker_mode: bool,
    master_mode: bool,
    config_file: PathBuf,
}

#[derive(Deserialize)]
struct FsRemap {
    master: String,
    worker: String,
}

#[derive(Deserialize)]
struct MasterConfig {
    orch_bind_addr: SocketAddr,
    web_bind_addr: SocketAddr,
}

#[derive(Deserialize)]
struct WorkerConfig {
    identifier: String,
    orch_addr: SocketAddr,
    fs_remaps: Option<Vec<FsRemap>>,
    cache_dir: PathBuf,
    handbrake_path: PathBuf,
    ffmpeg_path: PathBuf,
    exiftool_path: PathBuf,
    mediainfo_path: PathBuf,
    ccextractor_path: PathBuf,
    ffprobe_path: PathBuf,
    mkvpropedit_path: PathBuf,
}

#[derive(Deserialize)]
struct SystemConfig {
    #[serde(default)]
    master: MasterConfig,
    #[serde(default)]
    worker: WorkerConfig,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        WorkerConfig {
            identifier: String::from("worker"),
            orch_addr: "127.0.0.1:1849".parse().expect("Error setting orch_addr"),
            fs_remaps: None,
            cache_dir: PathBuf::from("./trahl-cache"),
            handbrake_path: PathBuf::from("handbrake"),
            ffmpeg_path: PathBuf::from("ffmpeg"),
            exiftool_path: PathBuf::from("exiftool"),
            mediainfo_path: PathBuf::from("mediainfo"),
            ccextractor_path: PathBuf::from("ccextractor"),
            ffprobe_path: PathBuf::from("ffprobe"),
            mkvpropedit_path: PathBuf::from("mkvpropedit"),
        }
    }
}

impl Default for MasterConfig {
    fn default() -> Self {
        MasterConfig {
            orch_bind_addr: "0.0.0.0:1849".parse().expect("Error setting orch_bind_addr"),
            web_bind_addr: "0.0.0.0:1850".parse().expect("Error setting web_bind_addr"),
        }
    }
}

fn main() -> Result<(), lexopt::Error> {
    let args = parse_args()?;
    let config = match parse_config(&args.config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config \"{}\": {}", &args.config_file.display(), e);
            std::process::exit(1);
        }
    };

    println!("args: {:#?}", args);
    Ok(())
}

fn parse_config(path: &PathBuf) -> Result<SystemConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: SystemConfig = toml::from_str(&contents)?;
    Ok(config)
}

fn print_usage() {
    let msg = r#"Usage: trahl [-m|--master] [-w|--worker] -c|--conf=file

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
            [master]
            orch_bind_addr="0.0.0.0:1849"
            web_bind_addr="0.0.0.0:1850"
            [worker]
            identifier="worker"
            orch_addr="127.0.0.1:1849"
            cache_dir="/tmp/trahl-cache"
            handbrake_path="handbrake"
            ffmpeg_path="ffmpeg"
            exiftool_path="exiftool"
            mediainfo_path="mediainfo"
            ccextractor_path="ccextractor"
            ffprobe_path="ffprobe"
            mkvpropedit_path="mkvpropedit"
            "#};

        write!(conf_file, "{}", conf_content)
            .expect("Failed to write to temporary file");

        assert_cmd::Command::cargo_bin("trahl")
            .unwrap()
            .arg("--master")
            .arg(format!("--conf={}", conf_file.path().to_string_lossy()))
            .assert()
            .success();
    }
    
    #[test]
    fn test_args_garbage() {
        let mut conf_file = NamedTempFile::new()
            .expect("Failed to create temporary file");

        let conf_content = indoc!{r#"
            [master]
            wathafuhckishthes huh?
            orch_bind_addr="0.0.0.0:1849"
            web_bind_addr="0.0.0.0:1850"
            [worker]
            identifier="worker"
            scrambleblebleorch_addr="127.0.0.1:1849"
            cache_dir="/tmp/trahl-cache"
            handbrake_path="handbrake"
            ffmpeg_path="ffmpeg"
            exiftool_path="exiftool"
            mediainfo_path="mediainfo"
            ccextractor_path="ccextractor"
            ffprobe_path="ffprobe"
            mkvpropedit_path="mkvpropedit"
            "#};

        write!(conf_file, "{}", conf_content)
            .expect("Failed to write to temporary file");

        assert_cmd::Command::cargo_bin("trahl")
            .unwrap()
            .arg("--master")
            .arg(format!("--conf={}", conf_file.path().to_string_lossy()))
            .assert()
            .failure();
    }
}
