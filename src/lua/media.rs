use mlua::{Error, Lua, LuaSerdeExt, Result, Table, Value, AnyUserData};
use tokio::process::Command;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{interval, Duration as TDuration};
use tracing::{error, info};

use crate::extcmd::ffprobe::ffprobe;
use crate::lua::OutChannelWrapper;
use crate::rpc::{JobStatusMsg, JobStatus, TranscodeProgress};

pub async fn _ffprobe(luactx: Lua, mediapath: String) -> Result<Value> {
    let cmdpath = PathBuf::from("ffprobe");
    let mediapath = PathBuf::from(mediapath);
    let json = ffprobe(&cmdpath, &mediapath)
        .await
        .map_err(Error::external)?;

    luactx.to_value(&json)
}

pub async fn _ffmpeg(luactx: Lua, args: Table) -> Result<Value> {
    let mut args_vec = Vec::new();
    let mut i = 1;
    while let Ok(val) = args.get::<String>(i) {
        args_vec.push(val);
        i += 1;
    }

    args_vec.push("-progress".to_string());
    args_vec.push("pipe:1".to_string());
    args_vec.push("-nostats".to_string());
    args_vec.push("-y".to_string());

    let cmdpath = PathBuf::from("ffmpeg");
    let mut child = Command::new(cmdpath)
        .args(&args_vec)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    info!("Started FFMPEG: {:?}", args_vec);

    let stdout = child.stdout.take().expect("Stdout is piped");
    //let stderr = child.stderr.take().expect("Stderr is piped");
    let mut reader = BufReader::new(stdout).lines();
    let mut block = HashMap::new();
    let mut heartbeat = interval(TDuration::from_secs(1));
    
    let job_id_str = luactx.named_registry_value::<String>("job_id")?;
    let job_id: u128 = job_id_str.parse().expect("Error parsing job_id");
    
    let ud = luactx.named_registry_value::<AnyUserData>("out_channel")?;
    let tx_wrapper = ud.borrow::<OutChannelWrapper>()?;

    loop {
        tokio::select! {
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Some((k,v)) = line.split_once('=') {
                            if k == "progress" {
                                let frame = block.get("frame").and_then(|f: &String| f.parse::<u64>().ok());
                                let fps = block.get("fps").and_then(|f: &String| f.parse::<u64>().ok());
                                let cur_time = block.get("fps").and_then(|f: &String| f.parse::<u64>().ok())
                                    .map(Duration::from_millis);

                                let tp = TranscodeProgress {
                                    frame: frame,
                                    fps: fps,
                                    cur_time: cur_time,
                                    percentage: None,
                                    eta: None,
                                };

                                let msg = JobStatusMsg {
                                    job_id: job_id,
                                    status: JobStatus::Progress(tp)
                                };
                                _ = tx_wrapper.tx.send(msg).await.map_err(Error::external);

                                block.clear();
                                if v == "end" {
                                    break;
                                }
                            } else {
                                block.insert(k.to_string(), v.to_string());
                            }
                        }
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        error!("FFMPEG stdout read error: {:?}", e);
                    }
                }
            }
            _ = heartbeat.tick() => {
                if !block.is_empty() {
                                let frame = block.get("frame").and_then(|f: &String| f.parse::<u64>().ok());
                                let fps = block.get("fps").and_then(|f: &String| f.parse::<u64>().ok());
                                let cur_time = block.get("fps").and_then(|f: &String| f.parse::<u64>().ok())
                                    .map(Duration::from_millis);

                                let tp = TranscodeProgress {
                                    frame: frame,
                                    fps: fps,
                                    cur_time: cur_time,
                                    percentage: None,
                                    eta: None,
                                };

                                let msg = JobStatusMsg {
                                    job_id: job_id,
                                    status: JobStatus::Progress(tp)
                                };
                    let _ = tx_wrapper.tx.send(msg).await.map_err(Error::external);
                }
            }
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        error!("FFMPEG exited with error: {:?}", status);
        return Ok(mlua::Value::Boolean(false));
    }
   
    Ok(mlua::Value::Boolean(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Lua, Result, Function, Table};
    use crate::tests::init_tracing;

    #[tokio::test]
    async fn test_ffprobe() -> Result<()> {
        init_tracing();
        let lua = Lua::new();
        let ffprobe_ffi = lua.create_async_function(_ffprobe)?;
        lua.globals().set("ffprobe", ffprobe_ffi)?;

        let lua_code = format!(r#"
            function test_ffprobe()
                local res = ffprobe("{}/{}")
                return res
            end
        "#, env!("CARGO_MANIFEST_DIR"), "test-resources/red_320x240_h264_1s.mp4");

        lua.load(lua_code).exec_async().await?;

        let test_fn: Function = lua.globals().get("test_ffprobe")?;
        let ret: Table = test_fn.call_async(()).await?;        

        Ok(())
    }
}
