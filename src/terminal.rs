use crossterm::{
    cursor, execute, queue, style,
    terminal::{self, ClearType},
    style::{Stylize, Color},
};
use std::io::{stdout, Write};
use std::time::SystemTime;
use crate::icons;

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

pub fn display_prompt(prefix: &str, query: &str, row: u16) {
    queue!(stdout(), cursor::MoveTo(0, row), style::Print(prefix)).unwrap();
    queue!(stdout(), cursor::MoveTo(prefix.len() as u16, row), style::Print(query)).unwrap();
    queue!(stdout(), cursor::Show, cursor::EnableBlinking).unwrap();
}

pub fn display_entry(name: &str, created: SystemTime, row: u16, selected: bool, max_width: usize, is_match: bool, nerd_fonts: bool) {
    let mut styled_name;
    let mut styled_created;

    let selected_fg = Color::Rgb{
        r: 242,
        g: 205,
        b: 205
    };
    let normal_fg = Color::Rgb{
        r:166,
        g:173,
        b:200
    };
    let match_fg = Color::Rgb{
        r:166,
        g: 227,
        b: 161
    };
    let selected_bg = Color::Rgb{
        r: 69,
        g: 71,
        b:90
    };

    if selected {
        queue!(stdout(), cursor::MoveTo(0, row), style::Print(">")).unwrap();
        styled_name = name.with(selected_fg).on(selected_bg);
        styled_created = print_time(created).with(selected_fg).on(selected_bg);
    } else {
        queue!(stdout(), style::ResetColor).unwrap();
        styled_name = name.with(normal_fg);
        styled_created = print_time(created).with(normal_fg);
    }

    if is_match {
        styled_name = styled_name.with(match_fg);
        styled_created = styled_created.with(match_fg);
    } else {
        queue!(stdout(), cursor::Hide).unwrap();
    }

    if nerd_fonts {
        let nerd_font_icon = icons::get_file_icon(name).with(normal_fg);
        queue!(stdout(), cursor::MoveTo(1, row), style::PrintStyledContent(nerd_font_icon)).unwrap();
    }

    queue!(stdout(), cursor::MoveTo(3, row), style::PrintStyledContent(styled_name)).unwrap();
    queue!(stdout(), cursor::MoveTo((max_width + 7) as u16, row), style::PrintStyledContent(styled_created)).unwrap();
}

pub fn flush() {
    stdout().flush().unwrap();
}

pub fn display_delete_warning(row: usize) {
    let warning = "Press d again to delete";
    let styled_warning = warning.with(Color::Rgb{
        r: 243,
        g: 139,
        b: 168
    }).italic();
    queue!(stdout(), cursor::MoveTo(50, row as u16), style::PrintStyledContent(styled_warning)).unwrap();
}
