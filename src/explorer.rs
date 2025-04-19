use std::io::{self, Write, BufWriter, IsTerminal};
use std::path::PathBuf;
use std::fs::File;
use crossterm::{terminal, execute};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::Color;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use mlua::Lua;

use crate::config::Config;
use crate::error::{Result, ExplorerError};
use crate::input::InputHandler;
use crate::state::AppState;
use crate::ui::Renderer;
use crate::theme::Theme;
use crate::lua::{create_rx_module, DisplayModuleFn, Entry};

pub struct FileExplorer {
    state: AppState,
    renderer: Renderer,
    lua: Lua,
    is_tty_mode: bool,
    dirty: bool,
}

impl FileExplorer {
    pub fn new(config: Config) -> Result<Self> {
        let lua = Self::init_lua()?;
        let display_modules = Self::setup_display_modules(&lua)?;
        let theme = Self::get_theme(&lua)?;
        
        Ok(Self {
            state: AppState::new(config, display_modules)?,
            renderer: Renderer::new(theme),
            lua,
            is_tty_mode: !std::io::stdout().is_terminal(),
            dirty: true,
        })
    }

    fn init_lua() -> Result<Lua> {
        let lua = Lua::new();
        let config_dir = dirs::config_dir().unwrap().join("rx").join("lua");
        let config_lua = config_dir.join("init.lua");
        
        // Setup Lua path
        let pkg: mlua::Table = lua.globals().get("package")
            .map_err(ExplorerError::LuaError)?;
        let old_path: String = pkg.get("path")
            .map_err(ExplorerError::LuaError)?;
        let new_path = format!(
            "{}/?.lua;{}/?/init.lua;{}",
            config_dir.display(), config_dir.display(), old_path
        );
        pkg.set("path", new_path)
            .map_err(ExplorerError::LuaError)?;

        // Create and set rx module
        let rx_module = create_rx_module(&lua)
            .map_err(ExplorerError::LuaError)?;
        lua.globals().set("rx", rx_module)
            .map_err(ExplorerError::LuaError)?;

        // Load config
        lua.load(&std::fs::read_to_string(config_lua)?)
            .exec()
            .map_err(ExplorerError::LuaError)?;

        Ok(lua)
    }

    fn setup_display_modules(lua: &Lua) -> Result<Vec<DisplayModuleFn>> {
        let rx_table: mlua::Table = lua.globals()
            .get("rx")
            .map_err(ExplorerError::LuaError)?;
        
        let modules_table: mlua::Table = rx_table
            .get("modules")
            .map_err(ExplorerError::LuaError)?;

        let mut display_modules = Vec::new();
        for pair in modules_table.pairs::<mlua::Value, mlua::Function>() {
            let (_, func) = pair.map_err(ExplorerError::LuaError)?;
            let lua_clone = lua.clone();
            let display_fn: DisplayModuleFn = Box::new(move |entry: &Entry| {
                let ud = lua_clone.create_userdata(entry.clone()).unwrap();
                func.call::<_>(ud).unwrap_or_default()
            });
            display_modules.push(display_fn);
        }
        
        Ok(display_modules)
    }

    fn get_theme(lua: &Lua) -> Result<Theme> {
        let rx_table: mlua::Table = lua.globals()
            .get("rx")
            .map_err(ExplorerError::LuaError)?;

        if let Ok(tbl) = rx_table.get::<_>("theme") {
            Theme::from_lua(&tbl).map_err(ExplorerError::LuaError)
        } else {
            Ok(Theme {
                fg: Color::White,
                bg: Color::Black,
                selected_fg: Color::Yellow,
                selected_bg: Color::DarkGrey,
                highlight: Color::Green,
            })
        }
    }

    fn setup_terminal(is_tty_mode: bool) -> Result<Box<dyn Write>> {
        let mut writer: Box<dyn Write> = if is_tty_mode {
            let tty = File::options().read(true).write(true).open("/dev/tty")?;
            Box::new(BufWriter::new(tty))
        } else {
            Box::new(io::stdout())
        };
        
        execute!(writer, EnterAlternateScreen, EnableMouseCapture)?;
        terminal::enable_raw_mode()?;
        
        Ok(writer)
    }

    fn cleanup_terminal<W: Write>(writer: &mut W) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(writer, LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }
    
    fn set_title<W: Write>(&self, writer: &mut W) {
        execute!(writer, SetTitle(format!(
            "rx - {}",
            self.state.current_path.display()
        ))).unwrap();
    }

    pub fn run(&mut self) -> Result<Option<PathBuf>> {
        let mut writer = Self::setup_terminal(self.is_tty_mode)?;
        self.set_title(&mut writer);

        let result = self.run_event_loop(&mut writer);
        
        Self::cleanup_terminal(&mut writer)?;
        
        result
    }
    
    fn run_event_loop<W: Write>(&mut self, writer: &mut W) -> Result<Option<PathBuf>> {
        loop {
            if self.dirty {
                self.renderer.render(writer, &self.state);
                self.dirty = false;
            }

            let event = crossterm::event::read()?;
            if let Some(path) = InputHandler::handle_event(event, &mut self.state, &mut self.renderer, writer)? {
                return Ok(Some(path));
            }
            
            self.dirty = true;
        }
    }
}