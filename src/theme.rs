use crossterm::style::Color;
use mlua::prelude::*;
use mlua::Table;

#[derive(Clone)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn from_lua(table: &LuaTable) -> LuaResult<Self> {
        let fg = table.get::<_>("fg")?;
        let bg = table.get::<_>("bg")?;
        let selected: Table = table.get::<_>("selected")?;
        let highlight = table.get::<_>("highlight")?;

        fn to_rgb(t: &LuaTable) -> LuaResult<Color> {
            let r = t.get::<_>("r")?;
            let g = t.get::<_>("g")?;
            let b = t.get::<_>("b")?;
            Ok(Color::Rgb { r, g, b })
        }

        Ok(Theme {
            fg: to_rgb(&fg)?,
            bg: to_rgb(&bg)?,
            selected_fg: to_rgb(&selected.get::<_>("fg")?)?,
            selected_bg: to_rgb(&selected.get::<_>("bg")?)?,
            highlight: to_rgb(&highlight)?,
        })
    }
}