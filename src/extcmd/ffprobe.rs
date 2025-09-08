use serde_json::Value as JsonValue;
use tokio::process::Command;
use std::{fmt, string::FromUtf8Error};
use std::path::PathBuf;

#[derive(Debug)]
pub enum FFProbeError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Utf8(std::string::FromUtf8Error),
    Failed(String),
}

impl fmt::Display for FFProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Json(e) => write!(f, "Json parse error: {}", e),
            Self::Utf8(e) => write!(f, "Utf8 parse error: {}", e),
            Self::Failed(e) => write!(f, "ffprobe failed: {}", e),
        }
    }
}

impl std::error::Error for FFProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Utf8(_) => None,
            Self::Failed(_) => None,
        }
    }
}

impl From<std::io::Error> for FFProbeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for FFProbeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<FromUtf8Error> for FFProbeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Utf8(value)
    }
}

pub async fn ffprobe(cmdpath: &PathBuf, mediapath: &PathBuf) -> Result<JsonValue, FFProbeError> {
    let cmd = Command::new(cmdpath)
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(mediapath)
        .output()
        .await?;

    if !cmd.status.success() {
        return Err(FFProbeError::Failed(
            String::from_utf8_lossy(&cmd.stderr).into()
        ));
    }

    let stdout = String::from_utf8(cmd.stdout)?;
    let json: JsonValue = serde_json::from_str(&stdout)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::{tempdir, TempDir};
    use tokio::process::Command;

    async fn create_example_media() -> (TempDir, PathBuf) {
        let dir = tempdir().expect("Failed to create tmpdir");
        let video_path: PathBuf = dir.path().join("test.mp4");
        let _ = Command::new("ffmpeg")
            .args(&[
                "-y",
                "-v", "quiet",
                "-f", "lavfi",
                "-i", "color=c=red:s=320x240:d=1",
                "-c:v", "libopenh264",
                "-t", "1",
                video_path.to_str().unwrap(),
            ])
            .status()
            .await
            .expect("Failed to create test media");

        (dir, video_path)
    }

    #[tokio::test]
    async fn test_ffprobe() -> Result<(), Box<dyn std::error::Error>> {
        let (_d, video) = create_example_media().await;
        let val = ffprobe(&PathBuf::from("ffprobe"), &video).await?;
        println!("{}", val);
        Ok(())
    }
}
