use serde::Deserialize;
use std::net::SocketAddr;
use std::path::{PathBuf};
use std::fs;
use toml;

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug)]
pub struct FsRemap {
    pub master: String,
    pub worker: String,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct MasterConfig {
    pub orch_bind_addr: SocketAddr,
    pub web_bind_addr: SocketAddr,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug)]
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

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
pub struct SystemConfig {
    #[serde(default)]
    pub master: MasterConfig,
    #[serde(default)]
    pub worker: WorkerConfig,
    #[serde(default)]
    pub log: LogConfig,
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

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".to_string(),
            file: None,
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            master: MasterConfig::default(),
            worker: WorkerConfig::default(),
            log: LogConfig::default(),
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
    use crate::config::{LogConfig, MasterConfig, SystemConfig, WorkerConfig};
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
    fn config_master_edited() {
        let mut conf_file = NamedTempFile::new().unwrap();
        let conf_content = indoc!{r#"
            [master]
            orch_bind_addr="0.0.0.0:1849"
            web_bind_addr="0.0.0.0:1859"
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        assert_ne!(config.master, MasterConfig::default());
        assert_eq!(config.worker, WorkerConfig::default());
        assert_eq!(config.log, LogConfig::default());
    }
    
    #[test]
    fn config_defaults() {
        let mut conf_file = NamedTempFile::new().unwrap();
        let conf_content = indoc!{r#"
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        assert_eq!(config.worker, WorkerConfig::default());
        assert_eq!(config.worker, WorkerConfig::default());
        assert_eq!(config.log, LogConfig::default());
    }
}
