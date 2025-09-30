use mlua::{Error, Lua, LuaSerdeExt, Result, Table, Value};
use std::path::PathBuf;

use crate::extcmd::ffprobe::ffprobe;

pub async fn _ffprobe(luactx: Lua, mediapath: String) -> Result<Value> {
    let cmdpath = PathBuf::from("ffprobe");
    let mediapath = PathBuf::from(mediapath);
    let json = ffprobe(&cmdpath, &mediapath)
        .await
        .map_err(Error::external)?;

    luactx.to_value(&json)
}

pub async fn _ffmpeg(luactx: Lua, args: Table) -> Result<Value> {
    todo!()
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
