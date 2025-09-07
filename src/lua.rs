mod http;
mod serialization;

use std::collections::HashMap;

use mlua::{Lua, Result, Table};
use tracing::{info, warn, error, debug};
use tokio::time::{sleep, Duration};

use serialization::_from_json;
use http::_http_request;

pub fn create_lua_context(vars: Option<HashMap<String, String>>) -> Result<Lua> {
    let luactx = Lua::new();
    let globals = luactx.globals();

    let table_trahl = luactx.create_table()?;
    let table_vars = luactx.create_table()?;

    create_ffis(&luactx, &table_trahl)?;
    if let Some(vars) = vars {
        create_vars(&luactx, &table_vars, vars)?;
    }

    globals.set("_trahl", &table_trahl)?;
    table_trahl.set("vars", table_vars)?;
    
    Ok(luactx)
}

fn create_ffis(luactx: &Lua, table: &Table) -> Result<()> {
    let ffi_log = luactx.create_function(_log)?;
    let ffi_delay_msec = luactx.create_async_function(_delay_msec)?;
    let ffi_http_request = luactx.create_async_function(_http_request)?;
    let ffi_from_json = luactx.create_function(_from_json)?;
    
    table.set("INFO", 1)?;
    table.set("WARN", 2)?;
    table.set("ERROR", 3)?;
    table.set("DEBUG", 4)?;
    table.set("log", ffi_log)?;
    
    table.set("delay_msec", ffi_delay_msec)?;
    table.set("http_request", ffi_http_request)?;
    table.set("from_json", ffi_from_json)?;
    
    Ok(())
}

fn create_vars(_luactx: &Lua, table: &Table, vars: HashMap<String, String>) -> Result<()> {
    for (key, value) in vars {
        table.set(key, value)?;
    }

    Ok(())
}

fn _log(_: &Lua, (level, msg): (u8, String)) -> Result<()> {
    match level {
        1u8 => info!(target: "lua", "{}", msg),
        2u8 => warn!(target: "lua", "{}", msg),
        3u8 => error!(target: "lua", "{}", msg),
        4u8 => debug!(target: "lua", "{}", msg),
        _ => info!(target: "lua", "{}", msg),
    }
    Ok(())
}

async fn _delay_msec(_: Lua, t: u64) -> Result<()> {
    let duration = Duration::from_millis(t);
    sleep(duration).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Result;
    use crate::tests::init_tracing;

    #[tokio::test]
    async fn test_log() -> Result<()> {
        init_tracing();
        let lua = create_lua_context(None)?;
        lua.load(r#"
            _trahl.log(_trahl.INFO, "INFO LOG")
            _trahl.log(_trahl.WARN, "WARN LOG")
            _trahl.log(_trahl.ERROR, "ERROR LOG")
            _trahl.log(_trahl.DEBUG, "DEBUG LOG")
        "#).exec_async().await?;

        Ok(())
    }
    
    #[tokio::test]
    async fn test_vars() -> Result<()> {
        init_tracing();
        let vars: HashMap<String, String> = HashMap::from([
            ("KEY_A".to_string(), "VAL_A".to_string()),
            ("KEY_B".to_string(), "123".to_string())
        ]);

        let lua = create_lua_context(Some(vars))?;

        lua.load(r#"
            assert(_trahl.vars.KEY_A == "VAL_A", "Wrong KEY_A value")
            assert(tonumber(_trahl.vars.KEY_B) == 123, "Wrong KEY_B value")
        "#).exec_async().await?;

        Ok(())
    }
}
