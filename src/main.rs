mod explorer;
mod terminal;

use std::io::stdout;
use crossterm::{cursor, queue};
use explorer::FileExplorer;

fn main() {
    let mut explorer = FileExplorer::new();
    if let Some(path) = explorer.run() {
        println!("{}", path.display());
    }
}