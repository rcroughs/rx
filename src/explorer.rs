use std::path::{Path, PathBuf};
use std::{env, io};
use std::fs::File;
use std::io::{stdout, BufWriter, IsTerminal, Write};
use std::cell::RefCell;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::{execute, queue, style, cursor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle, Clear, ClearType};
use crossterm::cursor::{Hide, Show, MoveTo};
use crossterm::style::Color;
use mlua::Lua;
use mlua::prelude::LuaTable;
use crate::config::Config;
use crate::terminal;
use crate::history::{Operation};
use crate::file_ops;
use crate::error::{ExplorerError, Result};
use crate::lua::{create_rx_module, DisplayModuleFn, Entry};
use crate::modes::{Mode, ModeAction};
use crate::prompt::Prompt;
use crate::theme::Theme;

// Add this wrapper struct that gives us a concrete Sized type
struct WriterAdapter<'a> {
    inner: &'a mut dyn Write,
}

impl<'a> Write for WriterAdapter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    prompt: Prompt,
    config: Config,
    delete_mode: Option<usize>,
    history: Vec<Operation>,
    history_index: usize,
    terminal_writer: RefCell<Option<Box<dyn Write>>>,
    is_tty_mode: bool,
    viewport_start: usize,
    viewport_size: usize,
    lua: Lua,
    display_modules: Vec<DisplayModuleFn>,
    theme: Theme,
    dirty: bool,
    modules_cache: Vec<Vec<String>>,
    max_widths: Vec<usize>,
}

impl FileExplorer {
    pub fn new(config: Config) -> Result<Self> {
        let lua = Lua::new();
        let config_dir = dirs::config_dir().unwrap().join("rx").join("lua");
        let config_lua = config_dir.join("config.lua");
        let pkg: LuaTable = lua.globals().get("package").map_err(|e| ExplorerError::LuaError(e))?;
        let old_path: String = pkg.get("path").map_err(|e| ExplorerError::LuaError(e))?;
        let new_path = format!(
            "{}/?.lua;{}/?/init.lua;{}",
            config_dir.display(), config_dir.display(), old_path
        );
        pkg.set("path", new_path)
            .map_err(|e| ExplorerError::LuaError(e))?;;
        let rx_module = create_rx_module(&lua).map_err(|e| ExplorerError::LuaError(e))?;
        lua.globals().set("rx", rx_module).map_err(|e| ExplorerError::LuaError(e))?;

        lua.load(&std::fs::read_to_string(config_lua)?).exec().map_err(|e| ExplorerError::LuaError(e))?;
        // Get the modules table
        let rx_table: mlua::Table = lua.globals().get("rx")
            .map_err(|e| ExplorerError::LuaError(e))?;

        let theme = if let Ok(tbl) = rx_table.get::<_>("theme") {
            Theme::from_lua(&tbl).map_err(ExplorerError::LuaError)?
        } else {
            Theme {
                fg: Color::White,
                bg: Color::Black,
                selected_fg: Color::Yellow,
                selected_bg: Color::DarkGrey,
                highlight: Color::Green,
            }
        };
        let modules_table: mlua::Table = rx_table.get("modules")
            .map_err(|e| ExplorerError::LuaError(e))?;
        let mut display_modules = Vec::new();

        // Convert each function in the table to a DisplayModuleFn
        for pair in modules_table.pairs::<mlua::Value, mlua::Function>() {
            let (_, func) = pair.map_err(|e| ExplorerError::LuaError(e))?;
            // clone the Lua context for the closure
            let lua_clone = lua.clone();
            let display_fn: DisplayModuleFn = Box::new(move |entry: &Entry| {
                // package the Rust Entry as Lua userdata
                let ud = lua_clone.create_userdata(entry.clone()).unwrap();
                // call the Lua function with the userdata
                func.call::<_>(ud).unwrap_or_default()
            });
            display_modules.push(display_fn);
        }

        let current_path = env::current_dir()?;
        let entries = file_ops::read_dir_entries(&current_path)?;
        
        // Check if we're in TTY mode
        let is_tty_mode = !stdout().is_terminal();
        
        let writer: Box<dyn Write> = if is_tty_mode {
            let tty = File::options().read(true).write(true).open("/dev/tty")?;
            Box::new(BufWriter::new(tty))
        } else {
            Box::new(stdout())
        };
        
        let mut me = FileExplorer {
            current_path,
            entries,
            selected: 1,
            prompt: Prompt::new(),
            config,
            delete_mode: None,
            history: vec![],
            history_index: 0,
            terminal_writer: RefCell::new(Some(writer)),
            is_tty_mode,
            viewport_start: 0,
            viewport_size: terminal::size_of_terminal().1 as usize,
            lua,
            display_modules,
            theme,
            dirty: true,
            modules_cache: Vec::new(),
            max_widths: Vec::new(),
        };
        me.recompute_display_data();
        Ok(me)
    }

    fn recompute_display_data(&mut self) {
        self.modules_cache.clear();
        for (idx, entry) in self.entries.iter().enumerate() {
            let info = self.create_entry(entry, self.get_display_name(entry, idx));
            let parts = self.display_modules
                .iter()
                .map(|m| m(&info))
                .collect();
            self.modules_cache.push(parts);
        }
        self.max_widths = vec![0; self.display_modules.len()];
        for parts in self.modules_cache.iter().skip(1) {
            for (i, s) in parts.iter().enumerate() {
                self.max_widths[i] = self.max_widths[i].max(s.len());
            }
        }
    }

    fn update_viewport(&mut self) {
        let terminal_height = terminal::size_of_terminal().1 as usize;
        self.viewport_size = terminal_height - 2;

        if self.selected >= self.viewport_start + self.viewport_size {
            self.viewport_start = self.selected - self.viewport_size + 1;
        } else if self.selected < self.viewport_start {
            self.viewport_start = self.selected;
        }
    }

    fn scroll_up(&mut self) {
        if self.viewport_start > 0 {
            self.viewport_start -= 1;
            if self.viewport_start + self.viewport_size <= self.selected {
                self.selected = self.viewport_start + self.viewport_size - 1;
            }
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn scroll_down(&mut self) {
        if self.viewport_start + self.viewport_size < self.entries.len() {
            self.viewport_start += 1;
            if self.selected < self.viewport_start {
                self.selected = self.viewport_start;
            }
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn create_entry(&self, entry: &PathBuf, display_name: String) -> Entry {
        Entry {
            path: entry.to_path_buf(),
            name: display_name,
            is_dir: entry.is_dir(),
            created: std::fs::metadata(entry)
                .and_then(|meta| meta.created())
                .unwrap_or_else(|_| std::time::SystemTime::now()),
        }
    }

    fn get_display_name(&self, entry: &PathBuf, index: usize) -> String {
        if index == 0 {
            "../".to_string()
        } else if entry.is_dir() {
            format!("{}/", entry.file_name().unwrap_or_default().to_string_lossy())
        } else {
            entry.file_name().unwrap_or_default().to_string_lossy().to_string()
        }
    }

    fn draw_row<W: Write>(
        &self,
        writer: &mut W,
        idx: usize,
        row: u16,
        modules: &[String],
    ) {
        let selected = idx == self.selected;
        let is_match = self.prompt.is_match(idx);
        terminal::display_entry(
            writer,
            modules.to_vec(),
            row,
            selected,
            self.max_widths.clone(),
            is_match,
            self.config.nerd_fonts,
            &self.theme,
        );
        if let Some(d) = self.delete_mode {
            if d == idx {
                terminal::display_delete_warning(writer, idx);
            }
        }
    }

    fn update_display(&mut self) {
        if !self.dirty {
            return;
        }
        self.with_writer(|writer| {
            queue!(
                writer,
                Hide,
                Clear(ClearType::All),
                MoveTo(0, 0),
            ).unwrap();

            let viewport_end = (self.viewport_start + self.viewport_size).min(self.entries.len());
            for (display_row, i) in (self.viewport_start..viewport_end).enumerate() {
                let modules = &self.modules_cache[i];
                self.draw_row(writer, i, display_row as u16, modules);
            }
            if self.prompt.is_active() {
                terminal::display_prompt(
                    writer,
                    self.prompt.get_prompt_prefix(),
                    self.prompt.get_query(),
                    terminal::size_of_terminal().0 - 1
                );
            }
            terminal::display_navbar(
                writer,
                self.viewport_start,
                viewport_end,
                self.entries.len()
            );

            queue!(writer, Show).unwrap();
            writer.flush().unwrap();
        });
        self.dirty = false;
    }

    fn navigate(&mut self) -> Result<()> {
        if self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            if selected_path.is_dir() {
                env::set_current_dir(&selected_path)?;
                self.current_path = env::current_dir()?;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
                self.selected = 1;
                self.set_title();
                self.viewport_start = 0;
                self.update_viewport();
                self.recompute_display_data();
                self.dirty = true;
            } else if !self.is_tty_mode {
                self.with_writer(|writer| terminal::cleanup(writer));
                file_ops::open_file_in_editor(selected_path)?;
                self.with_writer(|writer| terminal::init(writer));
            }
        }
        Ok(())
    }

    fn increment_selected(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected += 1;
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn decrement_selected(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn goto_header(&mut self) {
        if self.selected > 0 {
            self.selected = 0;
            self.viewport_start = 0;
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn goto_footer(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected = self.entries.len() - 1;
            self.update_viewport();
            self.dirty = true;
        }
    }

    fn delete(&mut self) -> Result<()> {
        if self.selected > 0 && self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            
            if self.delete_mode.is_none() {
                self.delete_mode = Some(self.selected);
                self.dirty = true;
                return Ok(());
            } 
            
            let operation = file_ops::prepare_delete_operation(selected_path, self.selected)?;
            
            file_ops::delete_path(selected_path, selected_path.is_dir())?;
            
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index);
            }
            
            self.history.push(operation);
            self.history_index += 1;
            self.entries.remove(self.selected);
            self.delete_mode = None;
            self.recompute_display_data();
            self.dirty = true;
        }
        Ok(())
    }

    fn undo(&mut self) -> Result<()> {
        if self.history_index > 0 {
            self.history_index -= 1;
            let operation = &self.history[self.history_index];

            match operation {
                Operation::Delete { path, is_dir, content, position, dir_backup } => {
                    file_ops::restore_deleted_path(path, *is_dir, content, dir_backup)?;
                    
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    if let Some(pos) = self.entries.iter().position(|p| p == path) {
                        self.selected = pos;
                    } else {
                        self.selected = *position.min(&(self.entries.len() - 1));
                    }
                },
                Operation::Create { path, is_dir } => {
                    file_ops::delete_path(path, *is_dir)?;
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    self.selected = self.selected.min(self.entries.len() - 1);
                }
                Operation::Rename { old_path, new_path } => {
                    file_ops::rename_path(new_path, old_path)?;
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    if let Some(pos) = self.entries.iter().position(|p| p == new_path) {
                        self.selected = pos;
                    } else {
                        self.selected = self.selected.min(self.entries.len() - 1);
                    }
                }
            }
            self.recompute_display_data();
            self.dirty = true;
        }
        Ok(())
    }

    fn redo(&mut self) -> Result<()> {
        if self.history_index < self.history.len() {
            let operation = &self.history[self.history_index].clone();

            match operation {
                Operation::Delete { path, is_dir, .. } => {
                    file_ops::delete_path(path, *is_dir)?;
                },
                Operation::Create { path, is_dir } => {
                    if *is_dir {
                        file_ops::create_directory(path)?;
                    } else {
                        file_ops::create_file(path)?;
                    }
                }
                Operation::Rename {
                    old_path,
                    new_path,
                } => {
                    file_ops::rename_path(old_path, new_path)?;
                }
            }

            self.history_index += 1;
            self.entries = file_ops::read_dir_entries(&self.current_path)?;

            if self.selected >= self.entries.len() {
                self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            }
            self.recompute_display_data();
            self.dirty = true;
        }
        Ok(())
    }

    fn set_title(&self) {
        self.with_writer(|writer| {
            let title = format!("rx - {}", self.current_path.display());
            execute!(writer, SetTitle(title)).unwrap();
        });
    }

    pub fn run(&mut self) -> Result<Option<PathBuf>> {
        enable_raw_mode()?;
        
        self.with_writer(|writer| {
            execute!(writer, EnterAlternateScreen).unwrap();
            terminal::init(writer);
        });
        self.set_title();
        self.update_display();
        let result = self.run_loop()?;
        
        self.with_writer(|writer| terminal::cleanup(writer));
        
        disable_raw_mode()?;
        self.with_writer(|writer| execute!(writer, LeaveAlternateScreen).unwrap());
        
        Ok(result)
    }

    fn run_loop(&mut self) -> Result<Option<PathBuf>> {
        loop {
            self.update_display();

            match event::read()? {
                Event::Key(key_event) => {
                    if let Some(path) = self.handle_key_event(key_event)? {
                        return Ok(Some(path));
                    }
                },
                Event::Mouse(mouse_event) => {
                    self.handle_mouse_event(mouse_event)?;
                },
                Event::Resize(_, _) => {
                    self.update_viewport();
                    self.dirty = true;
                },
                _ => {},
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<Option<PathBuf>> {
        if key_event.code != KeyCode::Char('d') {
            self.delete_mode = None;
        }

        if self.prompt.is_active() {
            match key_event.code {
                KeyCode::Esc => {
                    self.prompt.set_mode(Mode::Normal);
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Enter => {
                    let selected_path = self.entries.get(self.selected);
                    if let Some(action) = self.prompt.handle_input(
                        '\n',
                        &self.entries,
                        &self.current_path,
                        selected_path
                    )? {
                        self.handle_mode_action(action)?;
                    }
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char(c) => {
                    let selected_path = self.entries.get(self.selected);
                    if let Some(action) = self.prompt.handle_input(
                        c,
                        &self.entries,
                        &self.current_path,
                        selected_path
                    )? {
                        self.handle_mode_action(action)?;
                    }
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Backspace => {
                    let selected_path = self.entries.get(self.selected);
                    if let Some(action) = self.prompt.handle_input(
                        '\x7f',
                        &self.entries,
                        &self.current_path,
                        selected_path
                    )? {
                        self.handle_mode_action(action)?;
                    }
                    self.dirty = true;
                    Ok(None)
                },
                _ => Ok(None)
            }
        } else {
            match key_event.code {
                KeyCode::Char('/') => {
                    self.prompt.set_mode(Mode::Search);
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char('n') if !self.prompt.is_active() => {
                    if let Some(index) = self.prompt.next_match() {
                        self.selected = index;
                    }
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char('a') => {
                    self.prompt.set_mode(Mode::Create);
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char('r') if key_event.modifiers == event::KeyModifiers::CONTROL => {
                    self.redo()?;
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char('r') => {
                    if self.selected > 0 {
                        let name = self.entries[self.selected]
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        self.prompt.set_mode_with_text(Mode::Rename, &name);
                    }
                    self.dirty = true;
                    Ok(None)
                },
                KeyCode::Char('q') => {
                    self.with_writer(|writer| terminal::cleanup(writer));
                    Ok(Some(self.current_path.clone()))
                },
                KeyCode::Char('j') | KeyCode::Down => {
                    self.increment_selected();
                    Ok(None)
                },
                KeyCode::Char('k') | KeyCode::Up => {
                    self.decrement_selected();
                    Ok(None)
                },
                KeyCode::Char('G') | KeyCode::End => {
                    self.goto_footer();
                    Ok(None)
                },
                KeyCode::Char('g') | KeyCode::Home => {
                    self.goto_header();
                    Ok(None)
                },
                KeyCode::Char('d') => {
                    self.delete()?;
                    Ok(None)
                },
                KeyCode::Char('u') => {
                    self.undo()?;
                    Ok(None)
                },
                KeyCode::Enter => {
                    self.navigate()?;
                    Ok(None)
                },
                _ => Ok(None)
            }
        }
    }

    fn handle_mouse_event(&mut self, event: event::MouseEvent) -> Result<()> {
        let old_sel = self.selected;

        match event.kind {
            event::MouseEventKind::ScrollDown => {
                self.scroll_down();
            }
            event::MouseEventKind::ScrollUp => {
                self.scroll_up();
            }
            event::MouseEventKind::Down(event::MouseButton::Left) => {
                let row = event.row as usize;
                let idx = row + self.viewport_start;
                if idx < self.entries.len() {
                    if self.selected == idx {
                        self.navigate()?;
                    } else {
                        self.selected = idx;
                        self.update_viewport();
                    }
                }
            }
            _ => {}
        }

        self.dirty = true;
        Ok(())
    }

    fn handle_mode_action(&mut self, action: ModeAction) -> Result<()> {
        match action {
            ModeAction::Select(index) => {
                self.selected = index;
            },
            ModeAction::CreateEntry(operation) => {
                self.history.push(operation);
                self.history_index += 1;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
                self.selected = self.entries.len() - 1;
            },
            ModeAction::RenameEntry(operation) => {
                self.history.push(operation);
                self.history_index += 1;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
            },
            ModeAction::Exit => {},
        }
        self.recompute_display_data();
        self.dirty = true;
        self.update_viewport();
        Ok(())
    }

    // re-add writer helper
    fn with_writer<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut WriterAdapter) -> T,
    {
        let mut writer_opt = self.terminal_writer.borrow_mut();
        let writer = writer_opt.as_mut().expect("Writer should be available");
        let mut adapter = WriterAdapter { inner: writer.as_mut() };
        f(&mut adapter)
    }
}