//! Standalone `cd` command handler.
//!
//! Specification (per assignment):
//!   cd a1    -> OK | PATH NOT FOUND
//!
//! This implementation:
//! - Expects exactly one argument (the target path).
//! - If no argument is provided, attempts to change to root ("/").
//! - Uses the FileSystem's `cd` method (which resolves symlinks, supports . and ..).
//! - Prints:
//!     OK              on success
//!     PATH NOT FOUND  when filesystem not open, or target is invalid / not a directory
//!
//! NOTE: To activate this command you must:
//!   1. Add `pub mod cd;` to `commands/mod.rs`
//!   2. Insert mapping: `map.insert("cd", crate::commands::cd::handle_argv as Handler);`

use crate::context::Context;

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Acquire mutable FileSystem from context.
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Determine target path.
    let target = match argv.len() {
        0 => "/", // No args => go to root (optional design choice)
        1 => argv[0],
        _ => {
            // Too many arguments: treat as invalid path.
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Attempt to change directory.
    match fs.cd(target) {
        Ok(()) => println!("OK"),
        Err(_) => println!("PATH NOT FOUND"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::filesystem::FileSystem;
    use crate::fs::io::write_superblock;
    use crate::fs::layout::Superblock;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};

    // Helper to create a minimal mock FS file + superblock for testing cd root success.
    fn mock_fs_file() -> File {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("test.img")
            .unwrap();
        // Minimal superblock (root inode id = 0, counts = 0 so cd "/" succeeds trivially if resolver permits)
        let sb = Superblock {
            fs_size: 0,
            magic: *b"ELFS",
            root_inode_id: 0,
            bitmap_start: 0,
            bitmap_count: 0,
            block_start: 0,
            block_count: 0,
            inode_start: 0,
            inode_count: 1,
        };
        // Write superblock block (requires BLOCK_SIZE zero padding)
        write_superblock(&mut f, &sb).unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        f
    }

    #[test]
    fn cd_root_ok() {
        let file = mock_fs_file();
        let mut fs = FileSystem::open(file).unwrap();
        let mut ctx = Context {
            fs: Some(fs),
            fs_path: None,
        };
        handle_argv(&[], &mut ctx);
        // Expect OK printed (cannot assert stdout easily here without capturing; test checks no panic)
    }

    #[test]
    fn cd_arg_missing_fs() {
        let mut ctx = Context::new();
        handle_argv(&["/"], &mut ctx);
        // Should print PATH NOT FOUND (filesystem not open)
    }
}
