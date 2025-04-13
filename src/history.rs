use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::error::Result;

#[derive(Clone)]
pub enum Operation {
    Delete {
        path: PathBuf,
        is_dir: bool,
        content: Option<Vec<u8>>,
        position: usize,
        dir_backup: Option<DirBackup>,
    },
    Create {
        path: PathBuf,
        is_dir: bool,
    },
    Rename {
        old_path: PathBuf,
        new_path: PathBuf,
    },
}

impl Operation {
    pub fn clone(&self) -> Self {
        match self {
            Self::Delete { path, is_dir, content, position, dir_backup } => Self::Delete {
                path: path.clone(),
                is_dir: *is_dir,
                content: content.clone(),
                position: *position,
                dir_backup: dir_backup.clone(),
            },
            Self::Create { path, is_dir } => Self::Create {
                path: path.clone(),
                is_dir: *is_dir,
            },
            Self::Rename { old_path, new_path, } => Self::Rename {
                old_path: old_path.clone(),
                new_path: new_path.clone(),
            }
        }
    }
}

#[derive(Clone)]
pub struct DirBackup {
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub dirs: Vec<PathBuf>,
}

impl DirBackup {
    pub fn clone(&self) -> Self {
        Self {
            files: self.files.clone(),
            dirs: self.dirs.clone(),
        }
    }
}

fn backup_dir_contents(base_path: &Path, current_path: &Path, backup: &mut DirBackup) -> Result<()> {
    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base_path).unwrap_or(&path).to_path_buf();
        
        if path.is_dir() {
            backup.dirs.push(relative);
            backup_dir_contents(base_path, &path, backup)?;
        } else {
            let content = fs::read(&path)?;
            backup.files.insert(relative, content);
        }
    }
    Ok(())
}

pub fn backup_dir(path: &Path) -> Result<DirBackup> {
    let mut dir_backup = DirBackup {
        files: HashMap::new(),
        dirs: Vec::new(),
    };
    
    backup_dir_contents(path, path, &mut dir_backup)?;
    Ok(dir_backup)
}