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

pub fn size_of_terminal() -> (u16, u16) {
    let (width, height) = terminal::size().unwrap();
    (width, height)
}

pub fn display_search(query: &str, row: u16) {
    queue!(stdout(), cursor::MoveTo(0, row), style::Print("Search: ")).unwrap();
    queue!(stdout(), cursor::MoveTo(8, row), style::Print(query)).unwrap();
    queue!(stdout(), cursor::Show, cursor::EnableBlinking).unwrap();
}

pub fn display_entry(name: &str, created: SystemTime, row: u16, selected: bool, max_width: usize, is_match: bool) {
    let mut styled_name;
    let mut styled_created;
    if selected {
        queue!(stdout(), cursor::MoveTo(0, row), style::Print(">")).unwrap();
        styled_name = name.bold().bold();
        styled_created = print_time(created).bold();
    } else {
        queue!(stdout(), style::ResetColor).unwrap();
        styled_name = name.stylize();
        styled_created = print_time(created).stylize();
    }
    if is_match {
        styled_name = styled_name.green();
        styled_created = styled_created.green();
    } else {
        queue!(stdout(), cursor::Hide).unwrap();
    }
    queue!(stdout(), cursor::MoveTo(1, row), style::PrintStyledContent(styled_name)).unwrap();
    queue!(stdout(), cursor::MoveTo((max_width + 5) as u16, row), style::PrintStyledContent(styled_created)).unwrap();
}

pub fn flush() {
    stdout().flush().unwrap();
}