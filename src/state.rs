use std::path::PathBuf;
use crate::config::Config;
use crate::error::Result;
use crate::file_ops;
use crate::prompt::Prompt;
use crate::history::{Operation, DirBackup};
use crate::lua::{Entry, DisplayModuleFn};

pub struct AppState {
    pub current_path: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub prompt: Prompt,
    pub config: Config,
    pub delete_mode: Option<usize>,
    pub history: Vec<Operation>,
    pub history_index: usize,
    pub display_modules: Vec<DisplayModuleFn>,
    pub modules_cache: Vec<Vec<String>>,
    pub max_widths: Vec<usize>,
}

impl AppState {
    pub fn new(config: Config, display_modules: Vec<DisplayModuleFn>) -> Result<Self> {
        let current_path = std::env::current_dir()?;
        let entries = file_ops::read_dir_entries(&current_path)?;
        
        let mut state = Self {
            current_path,
            entries,
            selected: 1,
            prompt: Prompt::new(),
            config,
            delete_mode: None,
            history: vec![],
            history_index: 0,
            display_modules,
            modules_cache: Vec::new(),
            max_widths: Vec::new(),
        };
        state.recompute_display_data();
        Ok(state)
    }

    pub fn recompute_display_data(&mut self) {
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
}
