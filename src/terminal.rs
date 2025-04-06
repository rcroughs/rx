use crossterm::{
    cursor, execute, queue, style,
    terminal::{self, ClearType},
    style::Stylize,
};
use std::io::{stdout, Write};
use std::time::SystemTime;

pub fn init() {
    queue!(stdout(), cursor::Hide).unwrap();
    terminal::enable_raw_mode().unwrap();
}

pub fn cleanup() {
    queue!(stdout(), cursor::Show).unwrap();
    terminal::disable_raw_mode().unwrap();
}

pub fn clear_screen() {
    execute!(stdout(), terminal::Clear(ClearType::All)).unwrap();
}

fn print_time(created: SystemTime) -> String {
    let datetime = created
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = chrono::DateTime::from_timestamp(datetime as i64, 0).unwrap();
    time.format("%a %b %e %H:%M:%S %Y").to_string()
}

pub fn display_entry(name: &str, created: SystemTime, row: u16, selected: bool) {
    let styled_name;
    let styled_created;
    if selected {
        queue!(stdout(), cursor::MoveTo(0, row), style::Print(">")).unwrap();
        styled_name = name.bold().bold();
        styled_created = print_time(created).bold();
    } else {
        queue!(stdout(), style::ResetColor).unwrap();
        styled_name = name.stylize();
        styled_created = print_time(created).stylize();
    }
    queue!(stdout(), cursor::MoveTo(1, row), style::PrintStyledContent(styled_name)).unwrap();
    queue!(stdout(), cursor::MoveTo(16, row), style::PrintStyledContent(styled_created)).unwrap();
}

pub fn flush() {
    stdout().flush().unwrap();
}