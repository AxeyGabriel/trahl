mod http;
mod serialization;
mod time;
mod media;

use std::{collections::HashMap, sync::Weak};

use mlua::{AnyUserData, Error, Lua, LuaOptions, Result, StdLib, Table};
use tracing::{info, warn, error, debug};
use tokio::sync::mpsc;
use std::sync::Arc;

use serialization::_from_json;
use http::_http_request;
use time::{
    _delay_msec,
    _time,
};
use media::{
    _ffprobe,
    _ffmpeg,
};

use crate::rpc::JobStatusMsg;

const UTIL_LUA: &str = include_str!("../lualib/util.lua");

pub struct TrahlRuntimeCtx {
    status_tx: mpsc::Sender<JobStatusMsg>,
    job_id: u128,
}

impl TrahlRuntimeCtx {
    pub fn get_ref(lua: &Lua) -> Result<Arc<Self>> {
        let ud: AnyUserData = lua.named_registry_value("__trahl_runtime")?;
        let weak = ud.borrow::<Weak<Self>>()?;
        if let Some(arc) = weak.upgrade() {
            Ok(arc)
        } else {
            Err(Error::RuntimeError("Trahl runtime context dropped".into()))
        }
    }
}

pub struct TrahlRuntimeBuilder {
    vars: HashMap<String, String>,
    public: Arc<TrahlRuntimeCtx>,
    code: String,
}

impl TrahlRuntimeBuilder {
    pub fn new(job_id: u128, status_tx: mpsc::Sender<JobStatusMsg>, code: String) -> Self {
        Self {
            vars: HashMap::new(),
            public: Arc::new(TrahlRuntimeCtx {
                status_tx,
                job_id,
            }),
            code,
        }
    }

    pub fn add_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.vars.extend(vars);
        self
    }

    pub fn build(self) -> anyhow::Result<TrahlRuntime> {
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

        let public_vars = Arc::downgrade(&self.public);
        luactx.set_named_registry_value("__trahl_runtime", 
            luactx.create_any_userdata(public_vars)?
        )?;

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

        create_ffis(&luactx, &table_trahl)?;
        create_vars(&luactx, &table_vars, self.vars.clone())?;

        globals.set("_trahl", &table_trahl)?;
        table_trahl.set("vars", table_vars)?;

        Ok(TrahlRuntime {
            _public: self.public,
            luactx: luactx,
            code: self.code,
        })
    }
}

pub struct TrahlRuntime {
    _public: Arc<TrahlRuntimeCtx>,
    luactx: Lua,
    code: String,
}

impl TrahlRuntime {
    pub async fn exec(&self) -> anyhow::Result<()> {
        self.luactx.load(&self.code)
            .exec_async()
            .await?;
        Ok(())
    }

    pub fn get_output(&self) -> Result<String> {
        self.luactx.named_registry_value::<String>("output")
    }
    pub fn get_output_mode(&self) -> Result<u8> {
        self.luactx.named_registry_value::<u8>("output_mode")
    }
}

fn create_ffis(luactx: &Lua, table: &Table) -> Result<()> {
    let ffi_log = luactx.create_async_function(_log)?;
    let ffi_delay_msec = luactx.create_async_function(_delay_msec)?;
    let ffi_http_request = luactx.create_async_function(_http_request)?;
    let ffi_from_json = luactx.create_function(_from_json)?;
    let ffi_time = luactx.create_function(_time)?;
    let ffi_ffprobe = luactx.create_async_function(_ffprobe)?;
    let ffi_ffmpeg = luactx.create_async_function(_ffmpeg)?;
    let ffi_setoutput = luactx.create_async_function(_set_output)?;
    let ffi_milestone = luactx.create_async_function(_milestone)?;

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
    table.set("ffmpeg", ffi_ffmpeg)?;
    table.set("milestone", ffi_milestone)?;
    
    table.set("O_PRESERVE_DIR", 1)?;
    table.set("O_FLAT", 2)?;
    table.set("O_OVERWRITE", 3)?;
    table.set("set_output", ffi_setoutput)?;

    Ok(())
}

fn create_vars(_: &Lua, table: &Table, vars: HashMap<String, String>) -> Result<()> {
    for (key, value) in vars {
        table.set(key, value)?;
    }

    Ok(())
}

async fn _log(luactx: Lua, (level, msg): (u8, String)) -> Result<()> {
    match level {
        1u8 => info!(target: "lua", "{}", msg),
        2u8 => warn!(target: "lua", "{}", msg),
        3u8 => error!(target: "lua", "{}", msg),
        4u8 => debug!(target: "lua", "{}", msg),
        _ => info!(target: "lua", "{}", msg),
    }
    let runtimectx = TrahlRuntimeCtx::get_ref(&luactx)?.clone();
    let msg = JobStatusMsg::job_log(runtimectx.job_id, msg);
    runtimectx.status_tx.send(msg).await.map_err(Error::external)?;
    Ok(())
}

async fn _set_output(lua: Lua, (file, mode): (String, u8)) -> Result<()> {
    lua.set_named_registry_value("output", file)?;
    lua.set_named_registry_value("output_mode", mode)?;
    Ok(()) 
}

async fn _milestone(lua: Lua, descr: String) -> Result<()> {
    let runtimectx = TrahlRuntimeCtx::get_ref(&lua)?.clone();
    info!("JOB {}: new milestone: {}", runtimectx.job_id, descr);
    let msg = JobStatusMsg::job_milestone(runtimectx.job_id, descr);
    runtimectx.status_tx.send(msg).await.map_err(Error::external)?;
    Ok(()) 
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::init_tracing;
    use tokio::sync::mpsc;


    #[tokio::test]
    async fn test_log() -> anyhow::Result<()> {
        init_tracing();
        let (
            tx,
            mut rx
        ) = mpsc::channel::<JobStatusMsg>(10);
        
        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {}
        });

        
        let code = r#"
            _trahl.log(_trahl.INFO, "INFO LOG")
            _trahl.log(_trahl.WARN, "WARN LOG")
            _trahl.log(_trahl.ERROR, "ERROR LOG")
            _trahl.log(_trahl.DEBUG, "DEBUG LOG")
        "#;

        let lua = TrahlRuntimeBuilder::new(
            1,
            tx.clone(),
            code.to_string()
        ).build()?;

        lua.exec().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_vars() -> anyhow::Result<()> {
        init_tracing();
        let (
            tx,
            mut rx
        ) = mpsc::channel::<JobStatusMsg>(10);
        
        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {}
        });

        let vars: HashMap<String, String> = HashMap::from([
            ("KEY_A".to_string(), "VAL_A".to_string()),
            ("KEY_B".to_string(), "123".to_string())
        ]);
        
        let code = r#"
            assert(_trahl.vars.KEY_A == "VAL_A", "Wrong KEY_A value")
            assert(tonumber(_trahl.vars.KEY_B) == 123, "Wrong KEY_B value")
        "#;

        let lua = TrahlRuntimeBuilder::new(
            1,
            tx.clone(),
            code.to_string()
        )
        .add_vars(vars)
        .build()?;

        lua.exec().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_util_exists() -> anyhow::Result<()> {
        init_tracing();
        let (
            tx,
            mut rx
        ) = mpsc::channel::<JobStatusMsg>(10);

        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {}
        });
        
        let code = r#"
            local c = require("util")
        "#;

        let lua = TrahlRuntimeBuilder::new(
            1,
            tx.clone(),
            code.to_string()
        )
        .build()?;

        lua.exec().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_stdlibs() -> anyhow::Result<()> {
        init_tracing();
        let (
            tx,
            mut rx
        ) = mpsc::channel::<JobStatusMsg>(10);
        
        tokio::spawn(async move {
            while let Some(_) = rx.recv().await {}
        });
        
        let code = format!(r#"
        local c = require("util")
        local size = c.file_size("{}/{}")
        print("File size is " .. size .. "bytes")
        "#, env!("CARGO_MANIFEST_DIR"), "test-resources/100_bytes_file.bin");


        let lua = TrahlRuntimeBuilder::new(
            1,
            tx.clone(),
            code
        )
        .build()?;

        lua.exec().await?;

        Ok(())
    }
}
