use std::path::PathBuf;
use std::time::SystemTime;
use mlua::prelude::*;
use mlua::Table;
use crate::icons;

#[derive(Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub created: SystemTime,
}

pub type DisplayModuleFn = Box<dyn Fn(&Entry) -> String + 'static>;

pub fn get_icon(entry: &Entry) -> String {
    return icons::get_file_icon(entry.name.as_str()).to_string();
}

pub fn get_name(entry: &Entry) -> String {
    return entry.name.clone();
}

pub fn get_creation_date(entry: &Entry) -> String {
    let datetime = entry.created
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = chrono::DateTime::from_timestamp(datetime as i64, 0).unwrap();
    time.format("%a %b %e %H:%M:%S %Y").to_string()
}

pub fn create_rx_module<'lua>(lua: &'lua Lua) -> LuaResult<LuaTable> {
    let rx_table = lua.create_table()?;

    // make sure there's always a modules table, even before the user calls setDisplayModule
    rx_table.set("modules", lua.create_table()?)?;

    rx_table.set("Icon", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_icon(&entry))
    })?)?;
    rx_table.set("Name", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_name(&entry))
    })?)?;
    
    rx_table.set("CreationDate", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_creation_date(&entry))
    })?)?;

    rx_table.set("setDisplayModule", lua.create_function({
        let rx_table = rx_table.clone();
        move |lua_ctx, modules: LuaMultiValue| {
            let tbl = lua_ctx.create_table()?;
            for (i, module) in modules.into_iter().enumerate() {
                match module {
                    LuaValue::Function(f) => tbl.set(i + 1, f)?,
                    _ => return Err(LuaError::RuntimeError("Invalid module type".into())),
                }
            }
            rx_table.set("modules", tbl)?;
            Ok(())
        }
    })?)?;

    let rx_clone = rx_table.clone();
    let f = lua.create_function(move |_, theme_tbl: LuaTable| {
        rx_clone.set("theme", theme_tbl)?;
        Ok(())
    })?;
    rx_table.set("setTheme", f)?;


    Ok(rx_table)
}

impl LuaUserData for Entry {
    fn add_fields<'lua, F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("is_dir", |_, this| Ok(this.is_dir));
    }
}

