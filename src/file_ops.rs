use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{ExplorerError, Result};
use crate::history::{Operation, backup_dir, DirBackup};

pub fn read_dir_entries(path: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = vec![path.join("..")];
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(path)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else {
            files.push(path);
        }
    }

    dirs.sort_by_key(|a| a.file_name().unwrap_or_default().to_string_lossy().to_lowercase());
    files.sort_by_key(|a| a.file_name().unwrap_or_default().to_string_lossy().to_lowercase());

    entries.extend(dirs);
    entries.extend(files);
    Ok(entries)
}

pub fn delete_path(path: &Path, is_dir: bool) -> Result<()> {
    if is_dir {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn create_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::File::create(path)?;
    Ok(())
}

pub fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn open_file_in_editor(path: &Path) -> Result<()> {
    let status = Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|e| ExplorerError::OperationFailed(format!("Failed to open editor: {}", e)))?;
    
    if !status.success() {
        return Err(ExplorerError::OperationFailed("Editor exited with non-zero status".into()));
    }
    
    Ok(())
}

pub fn prepare_delete_operation(path: &Path, position: usize) -> Result<Operation> {
    let is_dir = path.is_dir();
    let content = if !is_dir {
        fs::read(path).ok()
    } else {
        None
    };

    let dir_backup = if is_dir {
        Some(backup_dir(path)?)
    } else {
        None
    };

    Ok(Operation::Delete {
        path: path.to_path_buf(),
        is_dir,
        content,
        position,
        dir_backup,
    })
}

pub fn restore_deleted_path(path: &Path, is_dir: bool, content: &Option<Vec<u8>>, dir_backup: &Option<DirBackup>) -> Result<()> {
    if is_dir {
        fs::create_dir_all(path)?;
        if let Some(backup) = dir_backup {
            // Restore directory contents from backup
            for (file_path, content) in &backup.files {
                let full_path = path.join(file_path);
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(full_path, content)?;
            }

            for dir_path in &backup.dirs {
                fs::create_dir_all(path.join(dir_path))?;
            }
        }
    } else if let Some(data) = content {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        // Restore the file with its content
        fs::write(path, data)?;
    }
    
    Ok(())
}

pub fn rename_path(old_path: &Path, new_path: &Path) -> Result<()> {
    fs::rename(old_path, new_path)?;
    Ok(())
}
