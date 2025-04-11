use std::path::PathBuf;

pub struct SearchState {
    pub query: String,
    pub mode: bool,
    pub matches: Vec<usize>,
    pub current_match: usize,
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            query: String::new(),
            mode: false,
            matches: Vec::new(),
            current_match: 0,
        }
    }
    
    pub fn reset(&mut self) {
        self.mode = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
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
    
    pub fn enter_search_mode(&mut self) {
        self.mode = true;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }
    
    pub fn handle_input(&mut self, c: char, entries: &[PathBuf]) -> Option<usize> {
        match c {
            '\n' => {
                self.mode = false;
                self.update_matches(entries);
                if !self.matches.is_empty() {
                    return Some(self.matches[self.current_match]);
                }
            },
            '\x7f' => { // Backspace
                self.query.pop();
            },
            c => {
                self.query.push(c);
            }
        }
        None
    }
}
