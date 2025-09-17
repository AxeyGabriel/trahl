mod http;
mod serialization;
mod time;
mod media;

use core::fmt;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mlua::{AnyUserData, Debug, DebugEvent, Error, HookTriggers, Lua, LuaOptions, Result, StdLib, Table, UserData, VmState};
use tracing::{info, warn, error, debug};
use tokio::sync::mpsc;

use serialization::_from_json;
use http::_http_request;
use time::{
    _delay_msec,
    _time,
};
use media::{
    _ffprobe,
};

struct OutChannelWrapper {
    tx: mpsc::Sender<String>,
}
impl UserData for OutChannelWrapper {}

struct Trace {
    timestamp: DateTime<Utc>,
    event: String,
    name: Option<String>,
    line: Option<usize>,
    current_line: Option<usize>,
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}][{}] {}: name: {}, line: {}",
            self.timestamp.to_rfc3339(),
            self.event,
            self.current_line.map_or("<none>".to_string(), |l| l.to_string()),
            self.name.as_deref().unwrap_or("<anonymous>"),
            self.line.map_or("<none>".to_string(), |l| l.to_string()),
        )
    }
}

const UTIL_LUA: &str = include_str!("../lualib/util.lua");

pub struct TrahlRuntime {
    vars: Option<HashMap<String, String>>,
    logs: Option<mpsc::Sender<String>>,
    tracing: Option<mpsc::Sender<Trace>>,
}

impl TrahlRuntime {
    pub fn new() -> Self {
        Self {
            vars: None,
            logs: None,
            tracing: None,
        }
    }

    pub fn with_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.vars = Some(vars);
        self
    }

    pub fn with_stdout(mut self, tx: mpsc::Sender<String>) -> Self {
        self.logs = Some(tx);
        self
    }

    pub fn with_tracing(mut self, tracing: mpsc::Sender<Trace>) -> Self {
        self.tracing = Some(tracing);
        self
    }

    pub fn build(&self) -> Result<Lua> {
        let luactx = Lua::new_with(
            StdLib::TABLE
            | StdLib::IO
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::UTF8
            | StdLib::PACKAGE,
            LuaOptions::default()
        )?;

        let _ = mlua_json::preload(&luactx);

        let globals = luactx.globals();
        let package: Table = globals.get("package")?;
        let preload: Table = package.get("preload")?;

        if let Some(tx) = &self.logs {
            luactx.set_named_registry_value("out_channel", OutChannelWrapper { tx: tx.clone() })?;
        }

        let register_module = |name: &str, code: &'static str| -> Result<()> {
            let loader = luactx.create_function(move |lua, ()| {
                let module: Table = lua.load(code).eval()?;
                Ok(module)
            })?;

            preload.set(name, loader)?;
            Ok(())
        };

        register_module("util", UTIL_LUA)?;

        let table_trahl = luactx.create_table()?;
        let table_vars = luactx.create_table()?;

        self.create_ffis(&luactx, &table_trahl)?;
        if let Some(vars) = &self.vars {
            self.create_vars(&luactx, &table_vars, vars.clone())?;
        }

        globals.set("_trahl", &table_trahl)?;
        table_trahl.set("vars", table_vars)?;

        if let Some(tracing) = &self.tracing {
            self.enable_tracing(&luactx, tracing.clone());
        }

        Ok(luactx)
    }

    fn create_ffis(&self, luactx: &Lua, table: &Table) -> Result<()> {
        //let ffi_log = luactx.create_function(_log)?;
        let ffi_log = luactx.create_async_function(async move |lua, (level, msg): (u8, String)| {
            match level {
                1u8 => info!(target: "lua", "{}", msg),
                2u8 => warn!(target: "lua", "{}", msg),
                3u8 => error!(target: "lua", "{}", msg),
                4u8 => debug!(target: "lua", "{}", msg),
                _ => info!(target: "lua", "{}", msg),
            }
            match lua.named_registry_value::<AnyUserData>("out_channel") {
                Ok(ud) => {
                    let tx_wrapper = ud.borrow::<OutChannelWrapper>()?;
                    tx_wrapper.tx.send(msg).await.map_err(Error::external);
                }
                Err(_) => {}
            }
            Ok(())
        })?;
        let ffi_delay_msec = luactx.create_async_function(_delay_msec)?;
        let ffi_http_request = luactx.create_async_function(_http_request)?;
        let ffi_from_json = luactx.create_function(_from_json)?;
        let ffi_time = luactx.create_function(_time)?;
        let ffi_ffprobe = luactx.create_async_function(_ffprobe)?;

        table.set("INFO", 1)?;
        table.set("WARN", 2)?;
        table.set("ERROR", 3)?;
        table.set("DEBUG", 4)?;
        table.set("log", ffi_log)?;

        table.set("delay_msec", ffi_delay_msec)?;
        table.set("http_request", ffi_http_request)?;
        table.set("from_json", ffi_from_json)?;
        table.set("time", ffi_time)?;
        table.set("ffprobe", ffi_ffprobe)?;

        Ok(())
    }

    fn create_vars(&self, _luactx: &Lua, table: &Table, vars: HashMap<String, String>) -> Result<()> {
        for (key, value) in vars {
            table.set(key, value)?;
        }

        Ok(())
    }

    fn enable_tracing(&self, luactx: &Lua, tx: mpsc::Sender<Trace>) -> Result<()> {
        let triggers = HookTriggers {
            on_calls: true,
            on_returns: false,
            every_line: false,
            every_nth_instruction: None,
        };

        let tx_clone = tx.clone();
        luactx.set_hook(triggers, move |_lua, dbg: &Debug| {
            let msg = Trace {
                event: match dbg.event() {
                    DebugEvent::Call => "CALL".to_string(),
                    DebugEvent::Ret => "RET".to_string(),
                    DebugEvent::TailCall => "TAILCALL".to_string(),
                    DebugEvent::Line => "LINE".to_string(),
                    DebugEvent::Count => "COUNT".to_string(),
                    DebugEvent::Unknown(n) => format!("UNKNOWN({})", n).to_string(),
                    
                },
                timestamp: Utc::now(),
                name: dbg.function().info().name,
                line: dbg.function().info().line_defined,
                current_line: dbg.current_line(),
            };
            let _ = tx_clone.try_send(msg);
            Ok(VmState::Continue)
        })?;

        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Result;
    use crate::tests::init_tracing;

    #[tokio::test]
    async fn test_log() -> Result<()> {
        init_tracing();

        let lua = TrahlRuntime::new().build()?;
        
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

        let lua = TrahlRuntime::new()
            .with_vars(vars)
            .build()?;

        lua.load(r#"
            assert(_trahl.vars.KEY_A == "VAL_A", "Wrong KEY_A value")
            assert(tonumber(_trahl.vars.KEY_B) == 123, "Wrong KEY_B value")
        "#).exec_async().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_util_exists() -> Result<()> {
        init_tracing();
        let lua = TrahlRuntime::new()
            .build()?;

        lua.load(r#"
            local c = require("util")
        "#).exec_async().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_stdlibs() -> Result<()> {
        init_tracing();
        let lua = TrahlRuntime::new()
            .build()?;

        lua.load(format!(r#"
        local c = require("util")
        local size = c.file_size("{}/{}")
        print("File size is " .. size .. "bytes")
        "#, env!("CARGO_MANIFEST_DIR"), "test-resources/100_bytes_file.bin")).exec_async().await?;

        Ok(())
    }
}
