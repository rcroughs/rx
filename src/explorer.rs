use std::path::{Path, PathBuf};
use std::env;
use crossterm::event::{self, Event, KeyCode};
use crate::config::Config;
use crate::terminal;
use crate::history::{Operation};
use crate::file_ops;
use crate::search::SearchState;
use crate::error::Result;

pub struct CreateState {
    pub mode: bool,
    pub query: String,
}

impl CreateState {
    pub fn new() -> Self {
        Self {
            mode: false,
            query: String::new(),
        }
    }
}

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    search: SearchState,
    config: Config,
    delete_mode: Option<usize>,
    create: CreateState,
    history: Vec<Operation>,
    history_index: usize,
}

impl FileExplorer {
    pub fn new(config: Config) -> Result<Self> {
        let current_path = env::current_dir()?;
        let entries = file_ops::read_dir_entries(&current_path)?;
        
        Ok(FileExplorer {
            current_path,
            entries,
            selected: 1,
            search: SearchState::new(),
            config,
            delete_mode: None,
            create: CreateState::new(),
            history: vec![],
            history_index: 0,
        })
    }

    fn display(&self) {
        terminal::clear_screen();

        for (i, entry) in self.entries.iter().enumerate() {
            self.display_entry(entry, i);
        }

        if self.search.mode {
            terminal::display_search(&self.search.query, terminal::size_of_terminal().0 - 1);
        }

        if self.create.mode {
            terminal::display_create(&self.create.query, terminal::size_of_terminal().0 - 1);
        }

        terminal::flush();
    }
    
    fn get_max_entry_width(&self) -> usize {
        self.entries
            .iter()
            .skip(1)
            .map(|entry| entry.file_name().unwrap_or_default().to_string_lossy().len())
            .max()
            .unwrap_or(0)
    }
    
    fn display_entry(&self, entry: &Path, index: usize) {
        let display_name = if index == 0 {
            "../".to_string()  // Special case for parent directory
        } else if entry.is_dir() {
            format!("{}/", entry.file_name().unwrap_or_default().to_string_lossy())
        } else {
            entry.file_name().unwrap_or_default().to_string_lossy().to_string()
        };

        let created = std::fs::metadata(entry)
            .and_then(|meta| meta.created())
            .unwrap_or_else(|_| std::time::SystemTime::now());

        let is_match = !self.search.query.is_empty() && self.search.matches.contains(&index);
        let max_width = self.get_max_entry_width();
        
        terminal::display_entry(&display_name, created, index as u16, 
            index == self.selected, max_width, is_match, self.config.nerd_fonts);
            
        if let Some(delete_index) = self.delete_mode {
            if delete_index == index {
                terminal::display_delete_warning(index);
            }
        }
    }

    fn navigate(&mut self) -> Result<()> {
        // Movement operations
        if self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            if selected_path.is_dir() {
                env::set_current_dir(&selected_path)?;
                self.current_path = env::current_dir()?;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
                self.selected = 1;
            } else {
                terminal::cleanup();
                file_ops::open_file_in_editor(selected_path)?;
                terminal::init();
            }
        }
        Ok(())
    }
    
    fn increment_selected(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    fn decrement_selected(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn goto_header(&mut self) {
        if self.selected > 0 {
            self.selected = 0;
        }
    }

    fn goto_footer(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected = self.entries.len() - 1;
        }
    }

    fn delete(&mut self) -> Result<()> {
        if self.selected > 0 && self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            
            // First call - just show warning
            if self.delete_mode.is_none() {
                self.delete_mode = Some(self.selected);
                return Ok(());
            } 
            
            // Second call - actually delete
            let operation = file_ops::prepare_delete_operation(selected_path, self.selected)?;
            
            file_ops::delete_path(selected_path, selected_path.is_dir())?;
            
            // Update history
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index);
            }
            
            self.history.push(operation);
            self.history_index += 1;
            self.entries.remove(self.selected);
            self.delete_mode = None;
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
                    
                    // Update the file list and selection
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
            }
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
            }

            self.history_index += 1;
            self.entries = file_ops::read_dir_entries(&self.current_path)?;

            if self.selected >= self.entries.len() {
                self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            }
        }
        Ok(())
    }

    fn enter_create_mode(&mut self) {
        self.create.mode = true;
        self.create.query.clear();
    }

    fn handle_create_input(&mut self, c: char) -> Result<()> {
        match c {
            '\n' => {
                if !self.create.query.is_empty() {
                    let path = self.current_path.join(&self.create.query);
                    let operation;
                    
                    if self.create.query.ends_with('/') {
                        file_ops::create_directory(&path)?;
                        operation = Operation::Create {path, is_dir: true};
                    } else {
                        file_ops::create_file(&path)?;
                        operation = Operation::Create {path, is_dir: false};
                    }
                    
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    self.selected = self.entries.len() - 1;
                    
                    if self.history_index < self.history.len() {
                        self.history.truncate(self.history_index);
                    }
                    
                    self.history.push(operation);
                    self.history_index += 1;
                }
                self.create.mode = false;
            },
            '\x7f' => { // Backspace
                self.create.query.pop();
            },
            _ => {
                self.create.query.push(c);
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<Option<PathBuf>> {
        terminal::init();

        loop {
            self.display();

            if let Event::Key(key_event) = event::read()? {
                // Reset delete mode if not pressing 'd' again
                if key_event.code != KeyCode::Char('d') {
                    self.delete_mode = None;
                }

                match key_event.code {
                    KeyCode::Char('/') if !self.search.mode && !self.create.mode => {
                        self.search.enter_search_mode();
                    },
                    KeyCode::Char('n') if !self.search.mode && !self.create.mode => {
                        if let Some(index) = self.search.next_match() {
                            self.selected = index;
                        }
                    },
                    KeyCode::Enter if self.search.mode => {
                        if let Some(index) = self.search.handle_input('\n', &self.entries) {
                            self.selected = index;
                        }
                    },
                    KeyCode::Enter if self.create.mode => {
                        self.handle_create_input('\n')?;
                    },
                    KeyCode::Backspace if self.search.mode => {
                        self.search.handle_input('\x7f', &self.entries);
                    },
                    KeyCode::Backspace if self.create.mode => {
                        self.handle_create_input('\x7f')?;
                    },
                    KeyCode::Esc if self.search.mode => {
                        self.search.reset();
                    },
                    KeyCode::Esc if self.create.mode => {
                        self.create.mode = false;
                        self.create.query.clear();
                    },
                    KeyCode::Char(c) if self.search.mode => {
                        self.search.handle_input(c, &self.entries);
                    },
                    KeyCode::Char(c) if self.create.mode => {
                        self.handle_create_input(c)?;
                    },
                    KeyCode::Char('q') => {
                        terminal::cleanup();
                        return Ok(None);
                    },
                    KeyCode::Char('j') | KeyCode::Down => self.increment_selected(),
                    KeyCode::Char('k') | KeyCode::Up => self.decrement_selected(),
                    KeyCode::Char('G') | KeyCode::End => self.goto_footer(),
                    KeyCode::Char('g') | KeyCode::Home => self.goto_header(),
                    KeyCode::Char('d') => self.delete()?,
                    KeyCode::Char('a') => self.enter_create_mode(),
                    KeyCode::Char('u') => self.undo()?,
                    KeyCode::Char('r') => self.redo()?,
                    KeyCode::Enter => self.navigate()?,
                    _ => {}
                }
            }
        }
    }
}