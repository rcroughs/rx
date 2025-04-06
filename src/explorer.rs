use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crossterm::event::{self, Event, KeyCode};
use crate::terminal;

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
}

impl FileExplorer {
    pub fn new() -> Self {
        let current_path = std::env::current_dir().unwrap();
        let entries = Self::read_dir_entries(&current_path);
        FileExplorer {
            current_path,
            entries,
            selected: 0,
        }
    }

    fn read_dir_entries(path: &Path) -> Vec<PathBuf> {
        fs::read_dir(path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect()
    }

    fn display(&self) {
        terminal::clear_screen();

        for (i, entry) in self.entries.iter().enumerate() {
            let name = entry.file_name().unwrap().to_string_lossy();
            let display_name = if entry.is_dir() {
                format!("{}/", name)
            } else {
                name.to_string()
            };

            terminal::display_entry(&display_name, i as u16, i == self.selected);
        }
        terminal::flush();
    }

    pub fn run(&mut self) -> Option<PathBuf> {
        terminal::init();

        loop {
            self.display();

            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('q') => {
                        terminal::cleanup();
                        return None;
                    }
                    KeyCode::Char('j') => {
                        if self.selected < self.entries.len() - 1 {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Char('k') => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        let selected_path = &self.entries[self.selected];
                        if selected_path.is_dir() {
                            terminal::cleanup();
                            return Some(selected_path.clone());
                        } else {
                            terminal::cleanup();
                            Command::new("nvim")
                                .arg(selected_path)
                                .status()
                                .unwrap();
                            terminal::init();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}