use crate::context::Context;
use std::io::{self, Write};

/// `cat s1`
///
/// Spec (assignment):
///   - Prints file content
///   - Errors: `FILE NOT FOUND`
///
/// Implementation notes:
/// - If the filesystem is not opened, prints `FILE NOT FOUND`.
/// - If the path is missing, or not exactly one argument, prints `FILE NOT FOUND`.
/// - Resolves the path (symlinks are already dereferenced by `resolve_path`).
/// - If target inode is not a regular file (`file_type != 0`), prints `FILE NOT FOUND`.
/// - Reads the entire file content and prints it as UTF-8 (lossy for non-UTF8).
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Require exactly one argument
    if argv.len() != 1 {
        eprintln!("FILE NOT FOUND");
        return;
    }
    let path = argv[0];

    // Get filesystem
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Resolve path
    let inode_id = match fs.resolve_path(path) {
        Ok(id) => id,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Read inode
    let inode = match fs.read_inode(inode_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Must be regular file (file_type == 0)
    if inode.file_type != 0 {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Empty file -> print nothing (still success)
    let size = inode.file_size as usize;
    if size == 0 {
        // Print empty line or nothing? Spec only says "OBSAH" => interpret as direct content.
        // We'll just print nothing (like standard `cat` on empty file).
        return;
    }

    // Stream content as lossy UTF-8 in chunks (avoid raw binary that UI může zahodit)
    let mut remaining = size;
    let mut offset: usize = 0;
    const CHUNK: usize = 64 * 1024;
    while remaining > 0 {
        let to_read = CHUNK.min(remaining);
        let mut chunk = vec![0u8; to_read];
        if let Err(_) = fs.read_file_range(&inode, offset as u64, &mut chunk) {
            eprintln!("FILE NOT FOUND");
            return;
        }
        let s = String::from_utf8_lossy(&chunk);
        eprint!("{}", s);
        remaining -= to_read;
        offset += to_read;
    }
    // Konec souboru – přidej newline
    eprintln!();
    let _ = std::io::stderr().flush();
}
