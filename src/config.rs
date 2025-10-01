use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::fs;
use toml;

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug, Clone)]
pub struct FsRemap {
    pub master: PathBuf,
    pub worker: PathBuf,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MasterConfig {
    pub orch_bind_addr: SocketAddr,
    pub web_bind_addr: SocketAddr,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug, Clone)]
pub struct JobConfig {
    pub name: String,
    pub enabled: bool,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub lua_script: PathBuf,
    pub variables: HashMap<String, String>,
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WorkerConfig {
    pub identifier: String,
    pub master_addr: SocketAddr,
    pub fs_remaps: Option<Vec<FsRemap>>,
    pub parallel_jobs: u8,
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
#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemConfig {
    #[serde(default)]
    pub master: MasterConfig,
    #[serde(default)]
    pub worker: WorkerConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub jobs: Vec<JobConfig>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        WorkerConfig {
            identifier: "worker".to_string(),
            master_addr: "127.0.0.1:1849".parse().expect("Error setting master_addr"),
            fs_remaps: None,
            cache_dir: PathBuf::from("./trahl-cache"),
            parallel_jobs: 1,
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
            jobs: Vec::new(),
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


impl FsRemap {
    pub fn map_to_master(&self, path: &Path) -> PathBuf {
        if let Ok(stripped) = path.strip_prefix(&self.worker) {
            self.master.join(stripped)
        } else {
            path.to_path_buf()
        }
    }

    pub fn map_to_worker(&self, path: &Path) -> PathBuf {
        if let Ok(stripped) = path.strip_prefix(&self.master) {
            self.worker.join(stripped)
        } else {
            path.to_path_buf()
        }
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
            scramblebleblemaster_addr="127.0.0.1:1849"
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
            [[jobs]]
            name = "Transcode Movies"
            enabled = true
            source_path = "/media/source/movies"
            destination_path = "/media/destination/movies"
            lua_script = "/configs/scripts/movie.lua"
            [jobs.variables]
            EXCLUDECODEC = "h265"

            [[jobs]]
            name = "Transcode TV Shows"
            enabled = true
            source_path = "/media/source/tv"
            destination_path = "/media/destination/tv"
            lua_script = "/configs/scripts/tv.lua"

            [jobs.variables]
            QUALITY = "720p"
            CODEC = "hevc"
            PRESET = "medium"
            "#};

        write!(conf_file, "{}", conf_content).unwrap();
        let path = conf_file.path().to_str().unwrap();
        let config = SystemConfig::parse(&PathBuf::from(path)).unwrap();

        let expected_jobs = vec![
            JobConfig {
                name: "Transcode Movies".to_string(),
                enabled: true,
                source_path: "/media/source/movies".into(),
                destination_path: "/media/destination/movies".into(),
                lua_script: "/configs/scripts/movie.lua".into(),
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
                variables: HashMap::from([
                    ("QUALITY".to_string(), "720p".to_string()),
                    ("CODEC".to_string(), "hevc".to_string()),
                    ("PRESET".to_string(), "medium".to_string()),
                ]),
            },
        ];

        assert_eq!(config.jobs, expected_jobs);
        assert_eq!(config.master, MasterConfig::default());
        assert_eq!(config.worker, WorkerConfig::default());
        assert_eq!(config.log, LogConfig::default());
    }
}
