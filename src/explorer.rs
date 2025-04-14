use std::path::{Path, PathBuf};
use std::{env, io};
use std::fs::File;
use std::io::{stdout, BufWriter, IsTerminal, Write};
use std::cell::RefCell;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use crate::config::Config;
use crate::terminal;
use crate::history::{Operation};
use crate::file_ops;
use crate::error::Result;
use crate::modes::{Mode, ModeAction};
use crate::prompt::Prompt;

// Add this wrapper struct that gives us a concrete Sized type
struct WriterAdapter<'a> {
    inner: &'a mut dyn Write,
}

impl<'a> Write for WriterAdapter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub struct FileExplorer {
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    prompt: Prompt,
    config: Config,
    delete_mode: Option<usize>,
    history: Vec<Operation>,
    history_index: usize,
    terminal_writer: RefCell<Option<Box<dyn Write>>>,
    is_tty_mode: bool,  // Add flag to track TTY mode
}

impl FileExplorer {
    pub fn new(config: Config) -> Result<Self> {
        let current_path = env::current_dir()?;
        let entries = file_ops::read_dir_entries(&current_path)?;
        
        // Check if we're in TTY mode
        let is_tty_mode = !stdout().is_terminal();
        
        let writer: Box<dyn Write> = if is_tty_mode {
            let tty = File::options().read(true).write(true).open("/dev/tty")?;
            Box::new(BufWriter::new(tty))
        } else {
            Box::new(stdout())
        };
        
        Ok(FileExplorer {
            current_path,
            entries,
            selected: 1,
            prompt: Prompt::new(),
            config,
            delete_mode: None,
            history: vec![],
            history_index: 0,
            terminal_writer: RefCell::new(Some(writer)),
            is_tty_mode,  // Store the TTY mode flag
        })
    }

    fn display<W: Write>(&self, writer: &mut W) {
        terminal::clear_screen(writer);

        for (i, entry) in self.entries.iter().enumerate() {
            self.display_entry(writer, entry, i);
        }

        if self.prompt.is_active() {
            terminal::display_prompt(
                writer,
                self.prompt.get_prompt_prefix(),
                self.prompt.get_query(),
                terminal::size_of_terminal().0 - 1
            );
        }

        terminal::flush(writer);
    }
    
    fn get_max_entry_width(&self) -> usize {
        self.entries
            .iter()
            .skip(1)
            .map(|entry| entry.file_name().unwrap_or_default().to_string_lossy().len())
            .max()
            .unwrap_or(0)
    }
    
    fn display_entry<W: Write>(&self, writer: &mut W, entry: &Path, index: usize) {
        let display_name = if index == 0 {
            "../".to_string()  // Special case for parent directory
        } else if entry.is_dir() {
            format!("{}/", entry.file_name().unwrap_or_default().to_string_lossy())
        } else {
            entry.file_name().unwrap_or_default().to_string_lossy().to_string()
        };

        let created = std::fs::metadata(entry)
            .and_then(|meta| meta.created())
            .unwrap_or_else(|_| std::time::SystemTime::now());

        let max_width = self.get_max_entry_width();
        let is_match = self.prompt.is_match(index);
        
        terminal::display_entry(writer, &display_name, created, index as u16, 
            index == self.selected, max_width, is_match, self.config.nerd_fonts);
            
        if let Some(delete_index) = self.delete_mode {
            if delete_index == index {
                terminal::display_delete_warning(writer, index);
            }
        }
    }

    fn with_writer<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut WriterAdapter) -> T,
    {
        let mut writer = self.terminal_writer.borrow_mut();
        let writer_ref = writer.as_mut().expect("Writer should be available");
        let mut adapter = WriterAdapter { inner: writer_ref.as_mut() };
        f(&mut adapter)
    }

    fn update_display(&self) {
        self.with_writer(|writer| {
            terminal::clear_screen(writer);

            for (i, entry) in self.entries.iter().enumerate() {
                self.display_entry(writer, entry, i);
            }

            if self.prompt.is_active() {
                terminal::display_prompt(
                    writer,
                    self.prompt.get_prompt_prefix(),
                    self.prompt.get_query(),
                    terminal::size_of_terminal().0 - 1
                );
            }

            terminal::flush(writer);
        });
    }

    fn navigate(&mut self) -> Result<()> {
        if self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            if selected_path.is_dir() {
                env::set_current_dir(&selected_path)?;
                self.current_path = env::current_dir()?;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
                self.selected = 1;
                self.set_title();
            } else if !self.is_tty_mode {
                // Only open files if not in TTY mode
                self.with_writer(|writer| terminal::cleanup(writer));
                file_ops::open_file_in_editor(selected_path)?;
                self.with_writer(|writer| terminal::init(writer));
            } else {
                // In TTY mode, show a message or simply do nothing
                // We could display a message here if needed
            }
        }
        Ok(())
    }
    
    fn increment_selected(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    fn decrement_selected(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn goto_header(&mut self) {
        if self.selected > 0 {
            self.selected = 0;
        }
    }

    fn goto_footer(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected = self.entries.len() - 1;
        }
    }

    fn delete(&mut self) -> Result<()> {
        if self.selected > 0 && self.selected < self.entries.len() {
            let selected_path = &self.entries[self.selected];
            
            if self.delete_mode.is_none() {
                self.delete_mode = Some(self.selected);
                return Ok(());
            } 
            
            let operation = file_ops::prepare_delete_operation(selected_path, self.selected)?;
            
            file_ops::delete_path(selected_path, selected_path.is_dir())?;
            
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index);
            }
            
            self.history.push(operation);
            self.history_index += 1;
            self.entries.remove(self.selected);
            self.delete_mode = None;
        }
        Ok(())
    }

    fn undo(&mut self) -> Result<()> {
        if self.history_index > 0 {
            self.history_index -= 1;
            let operation = &self.history[self.history_index];

            match operation {
                Operation::Delete { path, is_dir, content, position, dir_backup } => {
                    file_ops::restore_deleted_path(path, *is_dir, content, dir_backup)?;
                    
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    if let Some(pos) = self.entries.iter().position(|p| p == path) {
                        self.selected = pos;
                    } else {
                        self.selected = *position.min(&(self.entries.len() - 1));
                    }
                },
                Operation::Create { path, is_dir } => {
                    file_ops::delete_path(path, *is_dir)?;
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    self.selected = self.selected.min(self.entries.len() - 1);
                }
                Operation::Rename { old_path, new_path } => {
                    file_ops::rename_path(new_path, old_path)?;
                    self.entries = file_ops::read_dir_entries(&self.current_path)?;
                    if let Some(pos) = self.entries.iter().position(|p| p == new_path) {
                        self.selected = pos;
                    } else {
                        self.selected = self.selected.min(self.entries.len() - 1);
                    }
                }
            }
        }
        Ok(())
    }

    fn redo(&mut self) -> Result<()> {
        if self.history_index < self.history.len() {
            let operation = &self.history[self.history_index].clone();

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
                }
                Operation::Rename {
                    old_path,
                    new_path,
                } => {
                    file_ops::rename_path(old_path, new_path)?;
                }
            }

            self.history_index += 1;
            self.entries = file_ops::read_dir_entries(&self.current_path)?;

            if self.selected >= self.entries.len() {
                self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            }
        }
        Ok(())
    }

    fn set_title(&self) {
        self.with_writer(|writer| {
            let title = format!("rx - {}", self.current_path.display());
            execute!(writer, SetTitle(title)).unwrap();
        });
    }

    pub fn run(&mut self) -> Result<Option<PathBuf>> {
        enable_raw_mode()?;
        
        self.with_writer(|writer| {
            execute!(writer, EnterAlternateScreen).unwrap();
            terminal::init(writer);
        });
        self.set_title();
        let result = self.run_loop()?;
        
        self.with_writer(|writer| terminal::cleanup(writer));
        
        disable_raw_mode()?;
        self.with_writer(|writer| execute!(writer, LeaveAlternateScreen).unwrap());
        
        Ok(result)
    }

    fn run_loop(&mut self) -> Result<Option<PathBuf>> {
        loop {
            self.update_display();

            if let Event::Key(key_event) = event::read()? {
                if key_event.code != KeyCode::Char('d') {
                    self.delete_mode = None;
                }

                if self.prompt.is_active() {
                    match key_event.code {
                        KeyCode::Esc => self.prompt.set_mode(Mode::Normal),
                        KeyCode::Enter => {
                            let selected_path = self.entries.get(self.selected);
                            if let Some(action) = self.prompt.handle_input(
                                '\n',
                                &self.entries,
                                &self.current_path,
                                selected_path
                            )? {
                                self.handle_mode_action(action)?;
                            }
                        },
                        KeyCode::Char(c) => {
                            let selected_path = self.entries.get(self.selected);
                            if let Some(action) = self.prompt.handle_input(
                                c,
                                &self.entries,
                                &self.current_path,
                                selected_path
                            )? {
                                self.handle_mode_action(action)?;
                            }
                        },
                        KeyCode::Backspace => {
                            let selected_path = self.entries.get(self.selected);
                            if let Some(action) = self.prompt.handle_input(
                                '\x7f',
                                &self.entries,
                                &self.current_path,
                                selected_path
                            )? {
                                self.handle_mode_action(action)?;
                            }
                        },
                        _ => {}
                    }
                    continue;
                }

                match key_event.code {
                    KeyCode::Char('/') => {
                        self.prompt.set_mode(Mode::Search);
                    },
                    KeyCode::Char('n') if !self.prompt.is_active() => {
                        if let Some(index) = self.prompt.next_match() {
                            self.selected = index;
                        }
                    },
                    KeyCode::Char('a') => self.prompt.set_mode(Mode::Create),
                    KeyCode::Char('r') if key_event.modifiers == event::KeyModifiers::CONTROL => self.redo()?,
                    KeyCode::Char('r') => {
                        if self.selected > 0 {
                            let name = self.entries[self.selected]
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();
                            self.prompt.set_mode_with_text(Mode::Rename, &name);
                        }
                    },
                    KeyCode::Char('q') => {
                        self.with_writer(|writer| terminal::cleanup(writer));
                        break Ok(Some(self.current_path.clone()));
                    },
                    KeyCode::Char('j') | KeyCode::Down => self.increment_selected(),
                    KeyCode::Char('k') | KeyCode::Up => self.decrement_selected(),
                    KeyCode::Char('G') | KeyCode::End => self.goto_footer(),
                    KeyCode::Char('g') | KeyCode::Home => self.goto_header(),
                    KeyCode::Char('d') => self.delete()?,
                    KeyCode::Char('u') => self.undo()?,
                    KeyCode::Enter => self.navigate()?,
                    _ => {}
                }
            }
        }
    }

    fn handle_mode_action(&mut self, action: ModeAction) -> Result<()> {
        match action {
            ModeAction::Select(index) => {
                self.selected = index;
            },
            ModeAction::CreateEntry(operation) => {
                self.history.push(operation);
                self.history_index += 1;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
                self.selected = self.entries.len() - 1;
            },
            ModeAction::RenameEntry(operation) => {
                self.history.push(operation);
                self.history_index += 1;
                self.entries = file_ops::read_dir_entries(&self.current_path)?;
            },
            ModeAction::Exit => {},
        }
        Ok(())
    }
}