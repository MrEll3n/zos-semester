use crate::context::Context;
use crate::fs::consts::DIR_ENTRY_SIZE;
use crate::fs::layout::DirectoryEntry;

/// Standalone `ls` command.
///
/// Usage:
///   ls            -> lists current directory
///   ls <path>     -> lists specified directory or prints info for a single file
///
/// Output (per assignment):
///   - For directory entries:
///       "FILE: <name>"
///       "DIR: <name>"
///     Additionally (symlink extension):
///       "SYMLINK: <name>"
///   - On invalid path:
///       "PATH NOT FOUND"
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Obtain FileSystem instance.
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Determine target path: no args => current dir (".")
    let target = match argv.len() {
        0 => ".",
        1 => argv[0],
        _ => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve the path to an inode id.
    let inode_id = match fs.resolve_path(target) {
        Ok(id) => id,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Read the inode to decide if we list a directory or print a single entry.
    let inode = match fs.read_inode(inode_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Helper to derive last component (basename) for non-directory targets
    fn basename(path: &str) -> &str {
        let trimmed = path.trim_end_matches('/');
        let b = trimmed.rsplit('/').next().unwrap_or(trimmed);
        if b.is_empty() { "/" } else { b }
    }

    match inode.file_type {
        // Directory – list entries
        1 => {
            let slots = (inode.file_size as usize) / DIR_ENTRY_SIZE;
            let mut slot_buf = vec![0u8; DIR_ENTRY_SIZE];

            for i in 0..slots {
                if let Err(_) =
                    fs.read_file_range(&inode, (i * DIR_ENTRY_SIZE) as u64, &mut slot_buf)
                {
                    // If we fail to read a slot, skip it.
                    continue;
                }
                let entry = DirectoryEntry::deserialize(&slot_buf);
                if entry.is_unused() {
                    continue;
                }
                let name = entry.name_str();
                // Read child's inode to determine type
                match fs.read_inode(entry.inode_id) {
                    Ok(child) => match child.file_type {
                        0 => eprintln!("FILE: {}", name),
                        1 => eprintln!("DIR: {}", name),
                        2 => eprintln!("SYMLINK: {}", name),
                        _ => eprintln!("FILE: {}", name), // Fallback as regular file
                    },
                    Err(_) => {
                        // If child's inode can't be read, treat as not found (skip)
                        continue;
                    }
                }
            }
        }
        // Regular file – print single line
        0 => {
            let name = if argv.is_empty() {
                "."
            } else {
                basename(target)
            };
            eprintln!("FILE: {}", name);
        }
        // Symlink – print single line (visible in listing; if ls is called on a symlink path directly)
        2 => {
            let name = if argv.is_empty() {
                "."
            } else {
                basename(target)
            };
            eprintln!("SYMLINK: {}", name);
        }
        // Unknown type – treat as not found
        _ => eprintln!("PATH NOT FOUND"),
    }
}
