use mlua::{Error, Lua, LuaSerdeExt, Result, Value};
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
            function print_table(t)
                for k, v in pairs(t) do
                    if type(v) == "table" then
                        print(k .. ":")
                        print_table(v)
                    else
                        print(k, v)
                    end
                end
            end

            function test_ffprobe()
                local res = ffprobe("/home/axey/Videos/samples/MP4_1920_18MG.mp4")
                print_table(res)
                return res
            end
        "#);

        lua.load(lua_code).exec_async().await?;

        let test_fn: Function = lua.globals().get("test_ffprobe")?;
        let ret: Table = test_fn.call_async(()).await?;        

        Ok(())
    }
}
