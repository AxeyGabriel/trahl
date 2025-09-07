use mlua::{Error, Lua, Result, Table};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub async fn _http_request(_: Lua,
    (method_str, url, headers_t): (String, String, Option<Table>))
-> Result<(u16, String)>
{
    let method = method_str
        .parse::<reqwest::Method>()
        .map_err(Error::external)?;
    let mut headers = HeaderMap::new();

    if let Some(table) = headers_t {
        table.for_each(|k: String, v: String| {
            let hdr_name = HeaderName::from_bytes(k.as_bytes()).map_err(Error::external)?;
            let hdr_value = HeaderValue::from_str(&v).map_err(Error::external)?;
            headers.insert(hdr_name, hdr_value);
            Ok(())
        })?;
    }

    let client = reqwest::Client::new();
    let res = client
        .request(method, &url)
        .headers(headers)
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
                local status, body = http_request("GET", "{}/get", nil)
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

    #[tokio::test]
    async fn test_http_request_headers() -> Result<()> {
        init_tracing();
        let lua = Lua::new();
        let http_ffi = lua.create_async_function(_http_request)?;
        lua.globals().set("http_request", http_ffi)?;

        let lua_code = format!(r#"
            function test_request()
                local headers = {{
                    ["X-Test-Header"] = "abc",
                }}

                local status, body = http_request("GET", "{}/headers", headers)
                return status, body
            end
        "#, HTTPBIN);

        lua.load(lua_code).exec_async().await?;

        let test_fn: Function = lua.globals().get("test_request")?;
        let (status, body): (u16, String) = test_fn.call_async(()).await?;

        assert_eq!(status, 200);
        assert!(body.contains(r#"X-Test-Header": "abc"#));

        Ok(())
    }
}
