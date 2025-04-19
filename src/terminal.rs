use crossterm::{cursor, execute, queue, style, terminal::{self, ClearType}, style::{Stylize, Color}, event};
use std::io::Write;
use std::time::SystemTime;
use crossterm::style::StyledContent;
use crate::icons;
use crate::theme::Theme;

pub fn init<W: Write>(writer: &mut W) {
    queue!(writer, cursor::Hide, event::EnableMouseCapture).unwrap();
    terminal::enable_raw_mode().unwrap();
}

pub fn cleanup<W: Write>(writer: &mut W) {
    queue!(writer, cursor::Show, event::DisableMouseCapture).unwrap();
    terminal::disable_raw_mode().unwrap();
}

pub fn clear_screen<W: Write>(writer: &mut W) {
    execute!(writer, terminal::Clear(ClearType::All)).unwrap();
}

pub fn size_of_terminal() -> (u16, u16) {
    let (width, height) = terminal::size().unwrap();
    (width, height)
}

pub fn display_prompt<W: Write>(writer: &mut W, prefix: &str, query: &str, row: u16) {
    queue!(writer, cursor::MoveTo(0, row), style::Print(prefix)).unwrap();
    queue!(writer, cursor::MoveTo(prefix.len() as u16, row), style::Print(query)).unwrap();
    queue!(writer, cursor::Show, cursor::EnableBlinking).unwrap();
}

pub fn display_entry<W: Write>(
    writer: &mut W,
    display_modules: Vec<String>,
    row: u16,
    selected: bool,
    max_width: Vec<usize>,
    is_match: bool,
    nerd_fonts: bool,
    theme: &Theme,
) {
    let mut styled_modules: Vec<StyledContent<String>> = Vec::new();

    for module in display_modules {
        styled_modules.push(module.with(theme.fg));
    }

    if selected {
        queue!(writer, cursor::MoveTo(0, row), style::Print(">")).unwrap();
        styled_modules = styled_modules.into_iter().map(|module| {
            module.with(theme.selected_fg).on(theme.selected_bg)
        }).collect();
    } else {
        queue!(writer, style::ResetColor).unwrap();
        styled_modules = styled_modules.into_iter().map(|module| {
            module.with(theme.fg)
        }).collect();
    }

    if is_match {
        styled_modules = styled_modules.into_iter().map(|module| {
            module.with(theme.highlight)
        }).collect();
    } else {
        queue!(writer, cursor::Hide).unwrap();
    }


    let mut position = 2;
    for (i, module) in styled_modules.iter().enumerate() {
        let module_width = max_width[i];
        queue!(writer, cursor::MoveTo((position) as u16, row), style::PrintStyledContent(module.clone())).unwrap();
        position += module_width as u16 + 1;
    }
}

pub fn flush<W: Write>(writer: &mut W) {
    writer.flush().unwrap();
}

pub fn display_delete_warning<W: Write>(writer: &mut W, row: usize) {
    let warning = "Press d again to delete";
    let styled_warning = warning.with(Color::Rgb{
        r: 243,
        g: 139,
        b: 168
    }).italic();
    queue!(writer, cursor::MoveTo(50, row as u16), style::PrintStyledContent(styled_warning)).unwrap();
}

// Display a line at the right of the screen
pub fn display_navbar<W: Write>(writer: &mut W, start_viewport: usize, end_viewport: usize, total_entries: usize) {
    if end_viewport >= total_entries {
        return;
    }

    let (width, height) = terminal::size().unwrap();

    let start = ((start_viewport as f64 / total_entries as f64) * height as f64) as usize;
    let end = ((end_viewport as f64 / total_entries as f64) * height as f64) as usize;

    for i in start..end {
        queue!(writer, cursor::MoveTo(width - 1, i as u16), style::Print("┃")).unwrap();
    }
}