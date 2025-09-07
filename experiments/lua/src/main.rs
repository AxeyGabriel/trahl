use mlua::{Lua, Result};
use std::fs;

fn main() -> Result<()> {
    let lua = Lua::new();
    let globals = lua.globals();

    globals.set("myvar", "axey")?;

    let script = fs::read_to_string("plugin.lua").expect("read file failed");
    lua.load(&script).set_name("plugin.lua").exec()?;

    Ok(())
}
