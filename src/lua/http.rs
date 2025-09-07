use mlua::{Error, Lua, Result};

pub async fn _http_request(_: Lua, (method_str, url): (String, String)) -> Result<(u16, String)> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Lua, Result, Function};
    use crate::tests::init_tracing;
    
    const HTTPBIN: &str = "https://httpbin.org";

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
