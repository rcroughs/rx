use std::io::Write;
use crossterm::event::{Event, KeyEvent, MouseEvent, KeyCode, KeyModifiers, MouseEventKind, MouseButton};
use std::path::PathBuf;
use crate::error::Result;
use crate::state::AppState;
use crate::modes::{Mode, ModeAction};
use crate::file_ops;
use crate::history::Operation;
use crate::terminal;
use crate::ui::Renderer;

pub struct InputHandler;

impl InputHandler {
    pub fn handle_event<W: Write>(
        event: Event,
        state: &mut AppState,
        renderer: &mut Renderer,
        writer: &mut W,
    ) -> Result<Option<PathBuf>> {
        match event {
            Event::Key(key_event) => Self::handle_key_event(key_event, state, renderer, writer),
            Event::Mouse(mouse_event) => {
                Self::handle_mouse_event(mouse_event, state, renderer)?;
                Ok(None)
            },
            Event::Resize(_, _) => {
                renderer.update_viewport(state.selected, state.entries.len());
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    fn handle_key_event<W: Write>(
        key_event: KeyEvent,
        state: &mut AppState,
        renderer: &mut Renderer,
        writer: &mut W,
    ) -> Result<Option<PathBuf>> {
        // Reset delete mode unless pressing 'd' again
        if key_event.code != KeyCode::Char('d') {
            state.delete_mode = None;
        }

        if state.prompt.is_active() {
            Self::handle_prompt_input(key_event, state)
        } else {
            Self::handle_normal_input(key_event, state, renderer, writer)
        }
    }

    fn handle_prompt_input(
        key_event: KeyEvent,
        state: &mut AppState,
    ) -> Result<Option<PathBuf>> {
        match key_event.code {
            KeyCode::Esc => {
                state.prompt.set_mode(Mode::Normal);
                Ok(None)
            },
            KeyCode::Enter | KeyCode::Char(_) | KeyCode::Backspace => {
                let input = match key_event.code {
                    KeyCode::Enter => '\n',
                    KeyCode::Char(c) => c,
                    KeyCode::Backspace => '\x7f',
                    _ => unreachable!(),
                };

                let selected_path = state.entries.get(state.selected);
                if let Some(action) = state.prompt.handle_input(
                    input,
                    &state.entries,
                    &state.current_path,
                    selected_path,
                )? {
                    Self::handle_mode_action(action, state)?;
                }
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    fn handle_normal_input<W: Write>(
        key_event: KeyEvent,
        state: &mut AppState,
        renderer: &mut Renderer,
        writer: &mut W,
    ) -> Result<Option<PathBuf>> {
        match key_event.code {
            KeyCode::Char('/') => {
                state.prompt.set_mode(Mode::Search);
                Ok(None)
            },
            KeyCode::Char('n') => {
                if let Some(index) = state.prompt.next_match() {
                    state.selected = index;
                }
                Ok(None)
            },
            KeyCode::Char('a') => {
                state.prompt.set_mode(Mode::Create);
                Ok(None)
            },
            KeyCode::Char('r') if key_event.modifiers == KeyModifiers::CONTROL => {
                Self::redo(state)?;
                Ok(None)
            },
            KeyCode::Char('r') => {
                if state.selected > 0 {
                    let name = state.entries[state.selected]
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    state.prompt.set_mode_with_text(Mode::Rename, &name);
                }
                Ok(None)
            },
            KeyCode::Char('q') => {
                terminal::cleanup(writer);
                Ok(Some(state.current_path.clone()))
            },
            KeyCode::Char('j') | KeyCode::Down => {
                Self::increment_selected(state);
                renderer.update_viewport(state.selected, state.entries.len());
                Ok(None)
            },
            KeyCode::Char('k') | KeyCode::Up => {
                Self::decrement_selected(state);
                renderer.update_viewport(state.selected, state.entries.len());
                Ok(None)
            },
            KeyCode::Char('G') | KeyCode::End => {
                Self::goto_footer(state);
                renderer.update_viewport(state.selected, state.entries.len());
                Ok(None)
            },
            KeyCode::Char('g') | KeyCode::Home => {
                Self::goto_header(state);
                renderer.update_viewport(state.selected, state.entries.len());
                Ok(None)
            },
            KeyCode::Char('d') => {
                Self::handle_delete(state)?;
                Ok(None)
            },
            KeyCode::Char('u') => {
                Self::undo(state)?;
                Ok(None)
            },
            KeyCode::Enter | KeyCode::Right => {
                Self::navigate(state, renderer)?;
                Ok(None)
            },
            KeyCode::Left | KeyCode::Char('b') | KeyCode::Backspace => {
                Self::back(state);
                Ok(None)
            }
            _ => Ok(None)
        }
    }

    fn handle_mouse_event(
        event: MouseEvent,
        state: &mut AppState,
        renderer: &mut Renderer,
    ) -> Result<()> {
        match event.kind {
            MouseEventKind::ScrollDown => {
                renderer.scroll_down(state.entries.len());
                // Adjust selection if it's above viewport
                if state.selected < renderer.get_viewport_start() {
                    state.selected = renderer.get_viewport_start();
                }
            }
            MouseEventKind::ScrollUp => {
                renderer.scroll_up();
                // Adjust selection if it's below viewport
                let viewport_end = renderer.get_viewport_start() + renderer.get_viewport_size();
                if state.selected >= viewport_end {
                    state.selected = viewport_end - 1;
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let viewport_relative_row = event.row as usize;
                let absolute_row = renderer.get_viewport_start() + viewport_relative_row;
                if absolute_row < state.entries.len() {
                    if state.selected == absolute_row {
                        Self::navigate(state, renderer)?;
                    } else {
                        state.selected = absolute_row;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_mode_action(action: ModeAction, state: &mut AppState) -> Result<()> {
        match action {
            ModeAction::Select(index) => {
                state.selected = index;
            },
            ModeAction::CreateEntry(operation) => {
                state.history.push(operation);
                state.history_index += 1;
                state.entries = file_ops::read_dir_entries(&state.current_path)?;
                state.selected = state.entries.len() - 1;
            },
            ModeAction::RenameEntry(operation) => {
                state.history.push(operation);
                state.history_index += 1;
                state.entries = file_ops::read_dir_entries(&state.current_path)?;
            },
            ModeAction::Exit => {},
        }
        state.recompute_display_data();
        Ok(())
    }

    fn navigate(state: &mut AppState, renderer: &mut Renderer) -> Result<()> {
        if state.selected < state.entries.len() {
            let selected_path = &state.entries[state.selected];
            if selected_path.is_dir() {
                std::env::set_current_dir(selected_path)?;
                state.current_path = std::env::current_dir()?;
                state.entries = file_ops::read_dir_entries(&state.current_path)?;
                state.selected = 1;
                state.recompute_display_data();
                renderer.reset_viewport();
            } else {
                file_ops::open_file_in_editor(selected_path)?;
            }
        }
        Ok(())
    }

    fn handle_delete(state: &mut AppState) -> Result<()> {
        if state.selected > 0 && state.selected < state.entries.len() {
            let selected_path = &state.entries[state.selected];
            
            if state.delete_mode.is_none() {
                state.delete_mode = Some(state.selected);
                return Ok(());
            } 
            
            let operation = file_ops::prepare_delete_operation(selected_path, state.selected)?;
            
            file_ops::delete_path(selected_path, selected_path.is_dir())?;
            
            if state.history_index < state.history.len() {
                state.history.truncate(state.history_index);
            }
            
            state.history.push(operation);
            state.history_index += 1;
            state.entries.remove(state.selected);
            state.delete_mode = None;
            state.recompute_display_data();
        }
        Ok(())
    }

    fn undo(state: &mut AppState) -> Result<()> {
        if state.history_index > 0 {
            state.history_index -= 1;
            let operation = &state.history[state.history_index];

            match operation {
                Operation::Delete { path, is_dir, content, dir_backup, .. } => {
                    file_ops::restore_deleted_path(path, *is_dir, content, dir_backup)?;
                    state.entries = file_ops::read_dir_entries(&state.current_path)?;
                },
                Operation::Create { path, is_dir } => {
                    file_ops::delete_path(path, *is_dir)?;
                    state.entries = file_ops::read_dir_entries(&state.current_path)?;
                },
                Operation::Rename { old_path, new_path } => {
                    file_ops::rename_path(new_path, old_path)?;
                    state.entries = file_ops::read_dir_entries(&state.current_path)?;
                }
            }
            state.recompute_display_data();
        }
        Ok(())
    }

    fn redo(state: &mut AppState) -> Result<()> {
        if state.history_index < state.history.len() {
            let operation = &state.history[state.history_index].clone();

            match operation {
                Operation::Delete { path, is_dir, .. } => {
                    file_ops::delete_path(path, *is_dir)?;
                },
                Operation::Create { path, is_dir } => {
                    if *is_dir {
                        file_ops::create_directory(path)?;
                    } else {
                        file_ops::create_file(path)?;
                    }
                },
                Operation::Rename { old_path, new_path } => {
                    file_ops::rename_path(old_path, new_path)?;
                }
            }

            state.history_index += 1;
            state.entries = file_ops::read_dir_entries(&state.current_path)?;
            state.recompute_display_data();
        }
        Ok(())
    }

    fn increment_selected(state: &mut AppState) {
        if state.selected < state.entries.len() - 1 {
            state.selected += 1;
        }
    }

    fn decrement_selected(state: &mut AppState) {
        if state.selected > 0 {
            state.selected -= 1;
        }
    }

    fn goto_header(state: &mut AppState) {
        if state.selected > 0 {
            state.selected = 0;
        }
    }

    fn goto_footer(state: &mut AppState) {
        if state.selected < state.entries.len() - 1 {
            state.selected = state.entries.len() - 1;
        }
    }

    fn back(state: &mut AppState) {
        let parent = state.current_path.parent();
        if let Some(parent) = parent {
            std::env::set_current_dir(parent).unwrap();
            state.current_path = std::env::current_dir().unwrap();
            state.entries = file_ops::read_dir_entries(&state.current_path).unwrap();
            state.selected = 1;
            state.recompute_display_data();
        }
    }
}
