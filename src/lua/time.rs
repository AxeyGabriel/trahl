use tokio::time::{sleep, Duration};
use mlua::{Lua, Result, Error};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn _delay_msec(_: Lua, t: u64) -> Result<()> {
    let duration = Duration::from_millis(t);
    sleep(duration).await;
    Ok(())
}

pub fn _time(_: &Lua, _: ()) -> Result<u64> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::external(e))?
        .as_secs();

    Ok(time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Lua, Result, Function};

    #[tokio::test]
    async fn test_unix_time() -> Result<()> {
        let lua = Lua::new();
        let time_ffi = lua.create_function(_time)?;
        lua.globals().set("time", time_ffi)?;

        let lua_code = r#"
            function test_time()
                local unixtime = time()
                return unixtime
            end
        "#;

        lua.load(lua_code).exec_async().await?;

        let test_fn: Function = lua.globals().get("test_time")?;
        let luatime: u64 = test_fn.call(())?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(luatime >= now-1 && luatime <= now+1);

        Ok(())
    }
}
