use std::io::Write;
use crossterm::{cursor, queue, style};
use crate::state::AppState;
use crate::theme::Theme;
use crate::terminal;
use crossterm::terminal::{Clear, ClearType};

pub struct Renderer {
    viewport_start: usize,
    viewport_size: usize,
    theme: Theme,
}

impl Renderer {
    pub fn new(theme: Theme) -> Self {
        let (_, height) = terminal::size_of_terminal();
        Self {
            viewport_start: 0,
            viewport_size: height as usize - 2,
            theme,
        }
    }

    pub fn update_viewport(&mut self, selected: usize, total_entries: usize) {
        let terminal_height = terminal::size_of_terminal().1 as usize;
        self.viewport_size = terminal_height - 2;

        // Adjust viewport when selection is out of view
        if selected >= self.viewport_start + self.viewport_size {
            self.viewport_start = selected - self.viewport_size + 1;
        } else if selected < self.viewport_start {
            self.viewport_start = selected;
        }

        // Ensure viewport_start doesn't exceed possible bounds
        if self.viewport_start > total_entries.saturating_sub(self.viewport_size) {
            self.viewport_start = total_entries.saturating_sub(self.viewport_size);
        }
    }
    
    pub fn get_viewport_start(&self) -> usize {
        self.viewport_start
    }
    
    pub fn get_viewport_size(&self) -> usize {
        self.viewport_size
    }
    
    pub fn reset_viewport(&mut self) {
        self.viewport_start = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.viewport_start > 0 {
            self.viewport_start = self.viewport_start.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self, total_entries: usize) {
        let max_start = total_entries.saturating_sub(self.viewport_size);
        if self.viewport_start < max_start {
            self.viewport_start += 1;
        }
    }

    pub fn render<W: Write>(&self, writer: &mut W, state: &AppState) {
        queue!(
            writer,
            cursor::Hide,
            Clear(ClearType::All),
            cursor::MoveTo(0, 0),
        ).unwrap();

        let viewport_end = (self.viewport_start + self.viewport_size).min(state.entries.len());
        
        // Render entries
        for (display_row, i) in (self.viewport_start..viewport_end).enumerate() {
            self.draw_row(writer, state, i, display_row as u16);
        }

        // Render prompt if active
        if state.prompt.is_active() {
            terminal::display_prompt(
                writer,
                state.prompt.get_prompt_prefix(),
                state.prompt.get_query(),
                terminal::size_of_terminal().0 - 1
            );
        }

        // Render scrollbar
        terminal::display_navbar(
            writer,
            self.viewport_start,
            viewport_end,
            state.entries.len()
        );

        writer.flush().unwrap();
    }

    fn draw_row<W: Write>(
        &self,
        writer: &mut W,
        state: &AppState,
        idx: usize,
        row: u16,
    ) {
        let selected = idx == state.selected;
        let is_match = state.prompt.is_match(idx);
        let modules = &state.modules_cache[idx];

        terminal::display_entry(
            writer,
            modules.to_vec(),
            row,
            selected,
            state.max_widths.clone(),
            is_match,
            state.config.nerd_fonts,
            &self.theme,
        );

        if let Some(d) = state.delete_mode {
            if d == idx {
                terminal::display_delete_warning(writer, idx);
            }
        }
    }
}
