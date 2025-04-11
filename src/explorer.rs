use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crossterm::event::{self, Event, KeyCode};
use crate::config::Config;
use crate::terminal;

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    search_query: String,
    search_mode: bool,
    search_match: Vec<usize>,
    current_match: usize,
    config: Config,
}

impl FileExplorer {
    pub fn new(config: Config) -> Self {
        let current_path = std::env::current_dir().unwrap();
        let entries = Self::read_dir_entries(&current_path);
        FileExplorer {
            current_path,
            entries,
            selected: 1,
            search_query:    String::new(),
            search_mode:     false,
            search_match:    vec![],
            current_match: 0,
            config
        }
    }

    fn read_dir_entries(path: &Path) -> Vec<PathBuf> {
        let mut entries = vec![path.join("..")];
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in fs::read_dir(path).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }

        dirs.sort_by_key(|a| a.file_name().unwrap().to_string_lossy().to_lowercase());
        files.sort_by_key(|a| a.file_name().unwrap().to_string_lossy().to_lowercase());

        entries.extend(dirs);
        entries.extend(files);
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

            let is_match = !self.search_query.is_empty() && self.search_match.contains(&i);
            terminal::display_entry(&display_name, created, i as u16, i == self.selected, max_width, is_match, self.config.nerd_fonts);
        }

        if self.search_mode {
            terminal::display_search(&self.search_query, terminal::size_of_terminal().0 - 1);
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
                self.selected = 1;
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
                    KeyCode::Char('/') if !self.search_mode => self.enter_search_mode(),
                    KeyCode::Char('n') if !self.search_mode => self.next_search_match(),
                    KeyCode::Enter if self.search_mode => self.handle_search_input('\n'),
                    KeyCode::Backspace if self.search_mode => self.handle_search_input('\x7f'),
                    KeyCode::Esc => {
                        self.search_mode = false;
                        self.search_query.clear();
                        self.search_match.clear();
                    }
                    KeyCode::Char(c) if self.search_mode => self.handle_search_input(c),
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

    fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.search_match.clear();
        self.current_match = 0;
    }

    fn handle_search_input(&mut self, c: char) {
        if c == '\n' {
            self.search_mode = false;
            self.update_search_match();
            if !self.search_match.is_empty() {
                self.selected = self.search_match[self.current_match];
            }
        } else if c == '\x7f' { // Backspace
            self.search_query.pop();
        } else {
            self.search_query.push(c);
        }
    }

    fn update_search_match(&mut self) {
        self.search_match = self.entries.iter().skip(1).enumerate()
            .filter_map(|(i, entry)| {
            let name = entry.file_name().unwrap().to_string_lossy().to_lowercase();
            if name.contains(&self.search_query.to_lowercase()) {
                Some(i + 1)
            } else {
                None
            }
        })
        .collect();
    }

    fn next_search_match(&mut self) {
        if !self.search_match.is_empty() {
            self.current_match = (self.current_match + 1) % self.search_match.len();
            self.selected = self.search_match[self.current_match];
        }
    }
}