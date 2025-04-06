use crossterm::{
    cursor, execute, queue, style,
    terminal::{self, ClearType},
    style::Stylize,
};
use std::io::{stdout, Write};

pub fn init() {
    terminal::enable_raw_mode().unwrap();
}

pub fn cleanup() {
    terminal::disable_raw_mode().unwrap();
}

pub fn clear_screen() {
    execute!(stdout(), terminal::Clear(ClearType::All)).unwrap();
}

pub fn display_entry(name: &str, row: u16, selected: bool) {
    let content = if selected {
        name.magenta()
    } else {
        name.white()
    };
    queue!(stdout(), cursor::MoveTo(0, row), style::PrintStyledContent(content)).unwrap();
}

pub fn flush() {
    stdout().flush().unwrap();
}