use mlua::{Error, Lua, LuaSerdeExt, Result, Table, Value, AnyUserData};
use tokio::process::Command;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
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

pub async fn _ffmpeg(luactx: Lua, (duration, args): (f64, Table)) -> Result<()> {
    let mut args_vec = Vec::new();
    let mut i = 1;
    while let Ok(val) = args.get::<String>(i) {
        args_vec.push(val);
        i += 1;
    }

    let total_duration = Duration::from_secs_f64(duration);

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
    let stderr = child.stderr.take().expect("Stderr is piped");
    let mut err_reader = BufReader::new(stderr).lines();
    let mut reader = BufReader::new(stdout).lines();
    let mut block = HashMap::new();
    
    let job_id_str = luactx.named_registry_value::<String>("job_id")?;
    let job_id: u128 = job_id_str.parse().expect("Error parsing job_id");
    
    let ud = luactx.named_registry_value::<AnyUserData>("out_channel")?;
    let tx_wrapper = ud.borrow::<OutChannelWrapper>()?;

    loop {
        tokio::select! {
            line = err_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        let msg = JobStatusMsg {
                            job_id: job_id,
                            status: JobStatus::Log {
                                line: line
                            }
                        };
                        _ = tx_wrapper.tx.send(msg).await.map_err(Error::external);
                    },
                    Ok(None) => {}
                    Err(_) => {}
                }
            },
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Some((k,v)) = line.split_once('=') {
                            if k == "progress" {
                                let frame = block.get("frame")
                                    .and_then(|f: &String| f.parse::<u64>().ok());
                                let fps = block.get("fps")
                                    .and_then(|f: &String| f.parse::<f64>().ok())
                                    .map(|f| f.round() as u64);
                                let bitrate = block.get("bitrate")
                                    .and_then(|f: &String| f.trim().parse::<String>().ok());

                                //let total_size = block.get("total_size")
                                //    .and_then(|f: &String| f.parse::<u64>().ok());
                                let speed = block.get("speed")
                                    .and_then(|f: &String| f.trim().trim_end_matches('x').parse::<f64>().ok());
                                let cur_time = block.get("out_time_ms")
                                    .and_then(|f: &String| f.parse::<u64>().ok())
                                    .map(Duration::from_micros); //out_time_ms is actually
                                                                 //microseconds
                                let percentage = cur_time.map(|ct| {
                                    let pct = (ct.as_secs_f64() / total_duration.as_secs_f64()) * 100.0;
                                    pct.min(100.0).ceil()
                                });

                                let eta = cur_time.and_then(|ct| speed.map(|s| {
                                    let remaining = total_duration.saturating_sub(ct);
                                    Duration::from_secs_f64(remaining.as_secs_f64() / s)
                                }));

                                let tp = TranscodeProgress {
                                    frame: frame,
                                    fps: fps,
                                    cur_time: cur_time,
                                    percentage: percentage,
                                    eta: eta,
                                    bitrate: bitrate,
                                    speed: speed
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
        }
    }

    let status = child.wait()
        .await
        .map_err(|op| Error::external(op))?;

    if !status.success() {
        return Err(Error::external("ffmpeg failed"));
    }
   
    Ok(())
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
