use std::path::{Path, PathBuf};
use crate::modes::{Mode, ModeAction};
use crate::error::Result;
use crate::history::Operation;
use crate::file_ops;

pub struct Prompt {
    query: String,
    mode: Mode,
    matches: Vec<usize>,
    current_match: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            mode: Mode::Normal,
            matches: Vec::new(),
            current_match: 0,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }

    pub fn set_mode_with_text(&mut self, mode: Mode, text: &str) {
        self.mode = mode;
        self.query = text.to_string();
        self.matches.clear();
        self.current_match = 0;
    }

    pub fn is_active(&self) -> bool {
        self.mode != Mode::Normal
    }

    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }

    pub fn get_query(&self) -> &str {
        &self.query
    }

    pub fn get_prompt_prefix(&self) -> &str {
        match self.mode {
            Mode::Search => "Search: ",
            Mode::Create => "Create: ",
            Mode::Rename => "Rename: ",
            Mode::Normal => "",
        }
    }

    pub fn is_match(&self, index: usize) -> bool {
        self.mode == Mode::Search && self.matches.contains(&index)
    }

    fn handle_search(&mut self, input: char, entries: &[PathBuf]) -> Option<ModeAction> {
        match input {
            '\n' => {
                if !self.matches.is_empty() {
                    let selected = self.matches[self.current_match];
                    self.mode = Mode::Normal;
                    Some(ModeAction::Select(selected))
                } else {
                    self.mode = Mode::Normal;
                    Some(ModeAction::Exit)
                }
            },
            '\x08' | '\x7f' => { // Both Backspace variants
                self.query.pop();
                self.update_matches(entries);
                None
            },
            c => {
                self.query.push(c);
                self.update_matches(entries);
                None
            }
        }
    }

    fn handle_create(&mut self, input: char, current_path: &Path) -> Result<Option<ModeAction>> {
        match input {
            '\n' => {
                if self.query.is_empty() {
                    self.mode = Mode::Normal;
                    return Ok(Some(ModeAction::Exit));
                }

                let path = current_path.join(&self.query);
                let operation = if self.query.ends_with('/') {
                    file_ops::create_directory(&path)?;
                    Operation::Create { path, is_dir: true }
                } else {
                    file_ops::create_file(&path)?;
                    Operation::Create { path, is_dir: false }
                };

                self.mode = Mode::Normal;
                Ok(Some(ModeAction::CreateEntry(operation)))
            },
            '\x7f' => {
                self.query.pop();
                Ok(None)
            },
            c => {
                self.query.push(c);
                Ok(None)
            }
        }
    }

    fn handle_rename(&mut self, input: char, selected_path: &Path) -> Result<Option<ModeAction>> {
        match input {
            '\n' => {
                if self.query.is_empty() {
                    self.mode = Mode::Normal;
                    return Ok(Some(ModeAction::Exit));
                }

                let new_path = selected_path.parent().unwrap().join(&self.query);
                file_ops::rename_path(selected_path, &new_path)?;
                if selected_path != new_path {
                    let operation = Operation::Rename {
                        old_path: selected_path.to_path_buf(),
                        new_path,
                    };
                    self.mode = Mode::Normal;
                    return Ok(Some(ModeAction::RenameEntry(operation)));
                }
                self.mode = Mode::Normal;
                Ok(Some(ModeAction::Exit))
            },
            '\x7f' => {
                self.query.pop();
                Ok(None)
            },
            c => {
                self.query.push(c);
                Ok(None)
            }
        }
    }

    pub fn handle_input(&mut self, input: char, entries: &[PathBuf], current_path: &Path, selected_path: Option<&PathBuf>) -> Result<Option<ModeAction>> {
        match self.mode {
            Mode::Search => {
                if input == '\n' {
                    self.update_matches(entries);
                }
                Ok(self.handle_search(input, entries))
            },
            Mode::Create => self.handle_create(input, current_path),
            Mode::Rename => {
                if let Some(path) = selected_path {
                    self.handle_rename(input, path)
                } else {
                    Ok(Some(ModeAction::Exit))
                }
            },
            Mode::Normal => Ok(None),
        }
    }

    pub fn update_matches(&mut self, entries: &[PathBuf]) {
        self.matches = entries.iter().skip(1).enumerate()
            .filter_map(|(i, entry)| {
                let name = entry.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                
                if name.contains(&self.query.to_lowercase()) {
                    Some(i + 1)
                } else {
                    None
                }
            })
            .collect();
    }

    pub fn next_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        
        self.current_match = (self.current_match + 1) % self.matches.len();
        Some(self.matches[self.current_match])
    }
}
