use std::path::{Path, PathBuf};
use std::env;
use crossterm::event::{self, Event, KeyCode};
use crate::config::Config;
use crate::terminal;
use crate::history::{Operation};
use crate::file_ops;
use crate::error::Result;
use crate::modes::{Mode, ModeAction};
use crate::prompt::Prompt;

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    prompt: Prompt,
    config: Config,
    delete_mode: Option<usize>,
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
            prompt: Prompt::new(),
            config,
            delete_mode: None,
            history: vec![],
            history_index: 0,
        })
    }

    fn display(&self) {
        terminal::clear_screen();

        for (i, entry) in self.entries.iter().enumerate() {
            self.display_entry(entry, i);
        }

        if self.prompt.is_active() {
            terminal::display_prompt(
                self.prompt.get_prompt_prefix(),
                self.prompt.get_query(),
                terminal::size_of_terminal().0 - 1
            );
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

        let max_width = self.get_max_entry_width();
        let is_match = self.prompt.is_match(index);
        
        terminal::display_entry(&display_name, created, index as u16, 
            index == self.selected, max_width, is_match, self.config.nerd_fonts);
            
        if let Some(delete_index) = self.delete_mode {
            if delete_index == index {
                terminal::display_delete_warning(index);
            }
        }
    }

    fn navigate(&mut self) -> Result<()> {
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
            
            if self.delete_mode.is_none() {
                self.delete_mode = Some(self.selected);
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
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<Option<PathBuf>> {
        terminal::init();

        loop {
            self.display();

            if let Event::Key(key_event) = event::read()? {
                if key_event.code != KeyCode::Char('d') {
                    self.delete_mode = None;
                }

                if self.prompt.is_active() {
                    match key_event.code {
                        KeyCode::Esc => self.prompt.set_mode(Mode::Normal),
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
                        },
                        _ => {}
                    }
                    continue;
                }

                match key_event.code {
                    KeyCode::Char('/') => {
                        self.prompt.set_mode(Mode::Search);
                    },
                    KeyCode::Char('n') if !self.prompt.is_active() => {
                        if let Some(index) = self.prompt.next_match() {
                            self.selected = index;
                        }
                    },
                    KeyCode::Char('a') => self.prompt.set_mode(Mode::Create),
                    KeyCode::Char('r') if key_event.modifiers == event::KeyModifiers::CONTROL => self.redo()?,
                    KeyCode::Char('r') => {
                        if self.selected > 0 {
                            let name = self.entries[self.selected]
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();
                            self.prompt.set_mode_with_text(Mode::Rename, &name);
                        }
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
                    KeyCode::Char('u') => self.undo()?,
                    KeyCode::Enter => self.navigate()?,
                    _ => {}
                }
            }
        }
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
        Ok(())
    }
}