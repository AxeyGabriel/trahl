use mlua::{Error, Lua, LuaSerdeExt, Result, Value};

pub fn _from_json(luactx: &Lua, json_str: String) -> Result<Value> {
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(Error::external)?;

    luactx.to_value(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Lua, Result};
    use crate::tests::init_tracing;

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
        let lua_value = _from_json(&lua, json.to_string())?;
        lua.globals().set("my_json", lua_value)?;

        lua.load(r#"
            print(my_json.slideshow.title)
            print(my_json.slideshow.slides[1].title)
        "#).exec()?;

        Ok(())
    }
}
