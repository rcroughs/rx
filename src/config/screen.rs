use crossterm::{cursor, event, queue, style};
use std::io::{stdout};
use crossterm::event::{Event, KeyCode};
use crate::config::Config;
use crate::terminal;


pub struct ConfigScreen {
    titles: Vec<String>,
    subtitles: Vec<String>,
    index: usize,
    current_selection: bool,
    pub config: Config,
}

impl ConfigScreen {
    pub(crate) fn new(titles: Vec<String>, subtitles: Vec<String>) -> Self {
        ConfigScreen {
            titles,
            subtitles,
            index: 0,
            current_selection: true,
            config: Config{
                nerd_fonts: true,
            },
        }
    }

    fn display(&self) {
        terminal::clear_screen();
        self.show_title();
        self.show_subtitle();
        self.show_buttons();
        terminal::flush();
    }

    pub fn run(&mut self) {
        terminal::init();
        loop {
            self.display();
            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        if self.index == self.titles.len() - 1 {
                            return
                        } else {
                            match self.index {
                                0 => {
                                    self.config.nerd_fonts = self.current_selection;
                                }
                                _ => {}
                            }
                            self.index += 1;
                        }

                    }
                    KeyCode::Up | KeyCode::Down => {
                        self.current_selection = !self.current_selection;
                    }
                    _ => {}
                }
            }
        }
    }

    fn show_title(&self) {
        let (width, height) = terminal::size_of_terminal();
        let title = self.titles.get(self.index).unwrap();
        let padding = (width as usize - title.len()) / 2;
        let middle = height / 2;
        queue!(stdout(), cursor::MoveTo(padding as u16, middle - 2), style::Print(title)).unwrap();
    }

    fn show_subtitle(&self) {
        let (width, height) = terminal::size_of_terminal();
        let subtitle = self.subtitles.get(self.index).unwrap();
        let padding = (width as usize - subtitle.len()) / 2;
        let middle = height / 2;
        queue!(stdout(), cursor::MoveTo(padding as u16, middle - 1), style::Print(subtitle)).unwrap();
    }

    fn show_buttons(&self) {
        self.show_button("YES", self.current_selection, 1);
        self.show_button("NO", !self.current_selection, 2);
    }

    fn show_button(&self, text: &str, selected: bool, offset: i16) {
        let (width, height) = terminal::size_of_terminal();
        let button = text;
        let button = if selected {
            format!("> {} <", button)
        } else {
            format!("  {}  ", button)
        };
        let button_width = button.len() + 2;
        let padding = (width as usize - button_width) / 2;
        let middle = height / 2;
        if selected {
            queue!(stdout(), cursor::MoveTo(padding as u16, (middle as i16 + offset) as u16), style::Print(button)).unwrap();
        } else {
            queue!(stdout(), cursor::MoveTo(padding as u16, (middle as i16 + offset) as u16), style::Print(button)).unwrap();
        }
    }
}
