use std::io::stdout;
use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};

mod explorer;
mod terminal;
mod icons;
mod config;
mod history;
mod error;
mod file_ops;
mod modes;
mod prompt;

use explorer::FileExplorer;
use error::Result;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'o', long)]
    out: bool,
}


fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::get_config();
    let mut explorer = FileExplorer::new(config)?;
    if let Some(path) = explorer.run()? {
        if args.out {
            execute!(stdout(), Clear(ClearType::All)).unwrap();
            println!("{}\n", path.display());
        }
    }
    Ok(())
}