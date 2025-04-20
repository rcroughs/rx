use crate::history::Operation;

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Search,
    Create,
    Rename,
}

pub enum ModeAction {
    Select(usize),
    CreateEntry(Operation),
    RenameEntry(Operation),
    Exit,
}
