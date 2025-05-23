use std::path::PathBuf;
use std::time::SystemTime;
use mlua::prelude::*;
use crate::icons;

#[derive(Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub created: SystemTime,
    pub size: u64,
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

pub fn get_size(entry: &Entry) -> String {
    if entry.is_dir {
        return "".to_string();
    }
    let size = entry.size;
    if size < 1024 {
        format!("{:>3}  B", size)
    } else if size < 1024 * 1024 {
        format!("{:>3.2} KB", (size as f64 / 1024.0) as u64)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:>3.2} MB", (size as f64 / (1024.0 * 1024.0)) as u64)
    } else if size < 1024 * 1024 * 1024 * 1024 {
        format!("{:>3.2} GB", (size as f64 / (1024.0 * 1024.0 * 1024.0)) as u64)
    } else {
        format!("{:>3.2} TB", (size as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)) as u64)
    }
}

fn get_spacer(size: usize) -> String {
    let mut s = String::new();
    for _ in 0..size {
        s.push(' ');
    }
    s
}

pub fn get_small_spacer(entry: &Entry) -> String {
    get_spacer(2)
}

pub fn get_medium_spacer(entry: &Entry) -> String {
    get_spacer(4)
}

pub fn get_large_spacer(entry: &Entry) -> String {
    get_spacer(8)
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

    rx_table.set("Size", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_size(&entry))
    })?)?;

    rx_table.set("SmallSpacer", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_small_spacer(&entry))
    })?)?;

    rx_table.set("MediumSpacer", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_medium_spacer(&entry))
    })?)?;

    rx_table.set("LargeSpacer", lua.create_function(|_, entry: LuaAnyUserData| {
        let entry = entry.borrow::<Entry>()?;
        Ok(get_large_spacer(&entry))
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

pub fn default_display_modules(use_nerd_fonts: bool) -> Vec<DisplayModuleFn> {
    let mut display_modules: Vec<DisplayModuleFn> = Vec::new();
    if use_nerd_fonts {
        display_modules.push(Box::new(get_icon));
    }
    display_modules.push(Box::new(get_name));
    display_modules.push(Box::new(get_small_spacer));
    display_modules.push(Box::new(get_creation_date));
    display_modules.push(Box::new(get_size));
    display_modules.push(Box::new(get_small_spacer));
    display_modules
}

impl LuaUserData for Entry {
    fn add_fields<'lua, F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("is_dir", |_, this| Ok(this.is_dir));
        fields.add_field_method_get("created", |_, this| {
            let datetime = this.created
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Ok(datetime)
        });
        fields.add_field_method_get("size", |_, this| Ok(this.size));
    }
}

