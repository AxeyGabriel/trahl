use serde::Deserialize;
use std::net::SocketAddr;
use std::path::{PathBuf};
use std::fs;
use toml;

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
#[derive(Deserialize)]
pub struct FsRemap {
    pub master: String,
    pub worker: String,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct MasterConfig {
    pub orch_bind_addr: SocketAddr,
    pub web_bind_addr: SocketAddr,
}

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
#[derive(Deserialize)]
#[serde(default)]
pub struct WorkerConfig {
    pub identifier: String,
    pub orch_addr: SocketAddr,
    pub fs_remaps: Option<Vec<FsRemap>>,
    pub cache_dir: PathBuf,
    pub handbrake_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub exiftool_path: PathBuf,
    pub mediainfo_path: PathBuf,
    pub ccextractor_path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub mkvpropedit_path: PathBuf,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SystemConfig {
    pub master: Option<MasterConfig>,
    pub worker: Option<WorkerConfig>,
    pub log_level: String,
    pub log_file: PathBuf,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        WorkerConfig {
            identifier: "worker".to_string(),
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

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            master: None,
            worker: None,
            log_level: "info".to_string(),
            log_file: PathBuf::from("/dev/stdout"),
        }
    }
}

impl SystemConfig {
    pub fn parse(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {    
        let contents = fs::read_to_string(path)?;
        let config: SystemConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{SystemConfig, WorkerConfig};
    use tempfile::{NamedTempFile};
    use std::io::Write;
    use std::path::PathBuf;
    use indoc::indoc;

    #[test]
    fn config_garbage() {
        let mut conf_file = NamedTempFile::new().unwrap();
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

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path));

        match config {
            Ok(_) => panic!("Parser didn't fail"),
            _ => {}
        }
    }
    
    #[test]
    fn config_master() {
        let mut conf_file = NamedTempFile::new().unwrap();
        let conf_content = indoc!{r#"
            [master]
            orch_bind_addr="0.0.0.0:1849"
            web_bind_addr="0.0.0.0:1850"
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        assert!(config.master.is_some(), "Expected config.master to be Some, got None");
        assert!(config.worker.is_none(), "Expected config.worker to be None, got Some");
    }
    
    #[test]
    fn config_worker_defaults() {
        let mut conf_file = NamedTempFile::new().unwrap();
        let conf_content = indoc!{r#"
            [worker]
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        assert!(config.worker.is_some(), "Expected config.worker to be Some, got None");
        assert_eq!(config.worker.unwrap(), WorkerConfig::default());
    }
}
