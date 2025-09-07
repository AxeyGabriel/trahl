use mlua::{Error, Lua, LuaSerdeExt, Result, Value};
use tracing::{info, warn, error, debug};
use tokio::time::{sleep, Duration};

fn create_ffis(luactx: &Lua) -> Result<()> {
    let table_trahl = luactx.create_table()?;
    let ffi_log = luactx.create_function(_log)?;
    let ffi_delay_msec = luactx.create_async_function(_delay_msec)?;
    let ffi_http_request = luactx.create_async_function(_http_request)?;
    
    table_trahl.set("INFO", 1)?;
    table_trahl.set("WARN", 2)?;
    table_trahl.set("ERROR", 3)?;
    table_trahl.set("DEBUG", 4)?;
    table_trahl.set("log", ffi_log)?;
    
    table_trahl.set("delay_msec", ffi_delay_msec)?;
    table_trahl.set("http_request", ffi_http_request)?;
    luactx.globals().set("_trahl", table_trahl)?;
    
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

async fn _http_request(_: Lua, (method_str, url): (String, String)) -> Result<(u16, String)> {
    let method = method_str
        .parse::<reqwest::Method>()
        .map_err(Error::external)?;
    
    let client = reqwest::Client::new();
    let res = client
        .request(method, &url)
        .send()
        .await
        .map_err(Error::external)?;

    let status = res.status().as_u16();
    let body = res
        .text()
        .await
        .map_err(Error::external)?;

    Ok((status, body))
}

fn _to_json(luactx: &Lua, json_str: String) -> Result<Value> {
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(Error::external)?;

    luactx.to_value(&json)
}

async fn _delay_msec(_: Lua, t: u64) -> Result<()> {
    let duration = Duration::from_millis(t);
    sleep(duration).await;
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use mlua::{Lua, Result, Function};
    use tracing_subscriber::{fmt, EnvFilter};
    use std::sync::OnceLock;

    const HTTPBIN: &str = "https://httpbin.org";

    static TRACING: OnceLock<()> = OnceLock::new();

    fn init_tracing() {
        TRACING.get_or_init(|| {
            let filter = EnvFilter::from_str("debug").unwrap();
            let subscriber = fmt()
                .with_env_filter(filter)
                .finish();
            tracing::subscriber::set_global_default(subscriber).unwrap();
        });
    }

    #[tokio::test]
    async fn test_log() -> Result<()> {
        init_tracing();
        let lua = Lua::new();
        create_ffis(&lua).expect("Failed to set up lua ffi's");

        lua.load(r#"
            _trahl.log(_trahl.INFO, "INFO LOG")
            _trahl.log(_trahl.WARN, "WARN LOG")
            _trahl.log(_trahl.ERROR, "ERROR LOG")
            _trahl.log(_trahl.DEBUG, "DEBUG LOG")
        "#).exec_async().await?;

        Ok(())
    }
    
    #[tokio::test]
    async fn test_json() -> Result<()> {
        init_tracing();
        let json = serde_json::json!({
            "slideshow": {
                "title": "Sample Slide Show",
                "author": "Yours Truly", 
                "date": "date of publication", 
                "slides": [
                    {
                        "title": "Wake up to WonderWidgets!", 
                        "type": "all"
                    }, 
                    {
                        "items": [
                            "Why WonderWidgets are great", 
                            "Who buys WonderWidgets"
                        ], 
                        "title": "Overview", 
                        "type": "all"
                    },
                ], 
            }
        });

        let lua = Lua::new();
        let lua_value = _to_json(&lua, json.to_string())?;
        lua.globals().set("my_json", lua_value)?;

        lua.load(r#"
            print(my_json.slideshow.title)
            print(my_json.slideshow.slides[1].title)
        "#).exec()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_http_request() -> Result<()> {
        init_tracing();
        let lua = Lua::new();
        let http_ffi = lua.create_async_function(_http_request)?;
        lua.globals().set("http_request", http_ffi)?;

        let lua_code = format!(r#"
            function test_request()
                local status, body = http_request("GET", "{}/get")
                return status, body
            end
        "#, HTTPBIN);

        lua.load(lua_code).exec_async().await?;

        let test_fn: Function = lua.globals().get("test_request")?; 
        let (status, body): (u16, String) = test_fn.call_async(()).await?;
    
        assert_eq!(status, 200);
        assert!(body.contains(format!(r#""url": "{}/get""#, HTTPBIN).as_str()));

        Ok(())
    }
}
