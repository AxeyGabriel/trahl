use mlua::{Lua, Result, Error};
use regex::Regex;

pub fn _regex_match(_: &Lua, (text, pattern): (String, String)) -> Result<bool> {
    let re = Regex::new(&pattern).map_err(Error::external)?;
    Ok(re.is_match(&text))
}
