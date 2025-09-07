use serde::Deserialize;
use std::collections::HashMap;
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
    pub jobs: Vec<JobConfig>,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug)]
pub struct JobConfig {
    pub name: String,
    pub enabled: bool,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub lua_on_start: Option<PathBuf>,
    pub lua_script: PathBuf,
    pub lua_on_done: Option<PathBuf>,
    pub variables: HashMap<String, String>,
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
            jobs: Vec::new(),
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
    use crate::config::*;
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
    
    #[test]
    fn config_jobs() {
        let mut conf_file = NamedTempFile::new().unwrap();
        let conf_content = indoc!{r#"
            [[master.jobs]]
            name = "Transcode Movies"
            enabled = true
            source_path = "/media/source/movies"
            destination_path = "/media/destination/movies"
            lua_script = "/configs/scripts/movie.lua"
            [master.jobs.variables]
            EXCLUDECODEC = "h265"

            [[master.jobs]]
            name = "Transcode TV Shows"
            enabled = true
            source_path = "/media/source/tv"
            destination_path = "/media/destination/tv"
            lua_script = "/configs/scripts/tv.lua"

            [master.jobs.variables]
            QUALITY = "720p"
            CODEC = "hevc"
            PRESET = "medium"
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        let expected_master = MasterConfig {
            jobs: vec![
                JobConfig {
                    name: "Transcode Movies".to_string(),
                    enabled: true,
                    source_path: "/media/source/movies".into(),
                    destination_path: "/media/destination/movies".into(),
                    lua_script: "/configs/scripts/movie.lua".into(),
                    lua_on_done: None,
                    lua_on_start: None,
                    variables: HashMap::from([
                        ("EXCLUDECODEC".to_string(), "h265".to_string()),
                    ]),
                },
                JobConfig {
                    name: "Transcode TV Shows".to_string(),
                    enabled: true,
                    source_path: "/media/source/tv".into(),
                    destination_path: "/media/destination/tv".into(),
                    lua_script: "/configs/scripts/tv.lua".into(),
                    lua_on_done: None,
                    lua_on_start: None,
                    variables: HashMap::from([
                        ("QUALITY".to_string(), "720p".to_string()),
                        ("CODEC".to_string(), "hevc".to_string()),
                        ("PRESET".to_string(), "medium".to_string()),
                    ]),
                },
            ],
            ..MasterConfig::default()
        };

        assert_eq!(config.master, expected_master);
        assert_eq!(config.worker, WorkerConfig::default());
        assert_eq!(config.log, LogConfig::default());
    }
}
