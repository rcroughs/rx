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
        let mut entries = vec![path.join("..")];
        entries.extend(fs::read_dir(path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
        );
        entries
    }

    fn display(&self) {
        terminal::clear_screen();

        let max_width: usize = self.entries
            .iter()
            .skip(1)
            .map(|entry| entry.file_name().unwrap().to_string_lossy().len())
            .max()
            .unwrap_or(0);

        for (i, entry) in self.entries.iter().enumerate() {
            let display_name = if i == 0 {
                "../".to_string()  // Special case for parent directory
            } else if entry.is_dir() {
                format!("{}/", entry.file_name().unwrap().to_string_lossy())
            } else {
                entry.file_name().unwrap().to_string_lossy().to_string()
            };

            let created = fs::metadata(entry)
                .and_then(|meta| meta.created())
                .unwrap_or_else(|_| std::time::SystemTime::now());

            terminal::display_entry(&display_name, created, i as u16, i == self.selected, max_width);
        }
        terminal::flush();
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

    fn goto_selected(&mut self) {
        if self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            if selected_path.is_dir() {
                std::env::set_current_dir(&selected_path).unwrap();
                self.current_path = std::env::current_dir().unwrap();
                self.entries = Self::read_dir_entries(&self.current_path);
                self.selected = 0;
            } else {
                terminal::cleanup();
                Command::new("nvim")
                    .arg(selected_path)
                    .status()
                    .unwrap();
                terminal::init();
            }
        }
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
                    KeyCode::Char('j') | KeyCode::Down => self.increment_selected(),
                    KeyCode::Char('k') | KeyCode::Up => self.decrement_selected(),
                    KeyCode::Char('G') | KeyCode::End => self.goto_footer(),
                    KeyCode::Char('g') | KeyCode::Home => self.goto_header(),
                    KeyCode::Enter => self.goto_selected(),
                    _ => {}
                }
            }
        }
    }
}