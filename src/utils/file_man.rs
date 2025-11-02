use crate::commands::Context;
use std::path::Path;

pub fn handle_fs(fs_path: &str, context: &mut Context) {
    let path = Path::new(fs_path);

    if path.exists() && path.is_dir() {
        eprintln!(
            "Path points to a directory, cannot open it as a file: {}",
            fs_path
        );
        return;
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Cannot create directory '{}': {}", parent.display(), e);
                return;
            }
        }
    }

    let existed = path.exists();

    match context.open_fs(path) {
        Ok(()) => {
            if existed {
                eprintln!("Open existing file: {}", fs_path);
            } else {
                eprintln!("Creted new file: {}", fs_path);
            }
        }
        Err(e) => eprintln!("Failed to open/create '{}': {}", fs_path, e),
    }
}
