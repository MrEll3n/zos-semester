use crate::context::Context;
use std::fs::{OpenOptions, create_dir_all};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// outcp s1 s2
/// Copy a file from the custom filesystem (s1) to the host filesystem (s2).
///
/// Required outputs (per assignment):
///   OK
///   FILE NOT FOUND      (source file in FS neexistuje / není soubor)
///   PATH NOT FOUND      (neexistuje nebo nelze vytvořit cílová cesta / FS není otevřen / invalidní argumenty)
///
/// Semantics adopted here:
/// - If the filesystem is not opened -> PATH NOT FOUND
/// - If argument count is wrong -> PATH NOT FOUND
/// - If resolving source path fails or the inode is not a regular file -> FILE NOT FOUND
/// - If creating parent directory of host destination fails -> PATH NOT FOUND
/// - If opening or writing host destination fails -> PATH NOT FOUND
/// - Success -> OK
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate arguments
    if argv.len() != 2 {
        println!("PATH NOT FOUND");
        return;
    }
    let fs_src = argv[0];
    let host_dest = argv[1];

    // Acquire filesystem
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve source file inside FS
    let inode_id = match fs.resolve_path(fs_src) {
        Ok(id) => id,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Read inode and ensure it's a regular file (file_type == 0)
    let inode = match fs.read_inode(inode_id) {
        Ok(i) => i,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };
    if inode.file_type != 0 {
        // Not a regular file (directory or symlink target resolved to non-file)
        println!("FILE NOT FOUND");
        return;
    }

    // Prepare buffer for entire file
    let size = inode.file_size as usize;
    let mut buf = vec![0u8; size];
    if size > 0 {
        if let Err(_) = fs.read_file_range(&inode, 0, &mut buf) {
            println!("FILE NOT FOUND");
            return;
        }
    }

    // Ensure parent directory for host destination exists (if any)
    let host_path = PathBuf::from(host_dest);
    if let Some(parent) = host_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(_) = create_dir_all(parent) {
                println!("PATH NOT FOUND");
                return;
            }
        }
    }

    // Create/truncate destination file on host FS
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&host_path)
    {
        Ok(mut f) => {
            if let Err(_) = f.write_all(&buf) {
                println!("PATH NOT FOUND");
                return;
            }
        }
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    }

    println!("OK");
}

/// Optional helper (not used directly by handler): attempt to write a slice to host path.
/// Could be used if later you refactor outcp to stream instead of buffering whole file.
#[allow(dead_code)]
fn write_host_file(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            create_dir_all(parent)?;
        }
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    f.write_all(data)?;
    Ok(())
}
