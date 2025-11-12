use crate::context::Context;
use crate::fs::consts::DIR_ENTRY_SIZE;
use crate::fs::filesystem::FileSystem;
use crate::fs::layout::{DirectoryEntry, Inode};
use std::collections::HashSet;
use std::io;

// pwd command: prints the absolute path of the current working directory.
// Behavior:
// - No arguments expected. Extra arguments result in fallback to printing current path anyway.
// - If filesystem is not opened, prints "/".
// - Reconstructs the path by DFS from root to the current directory inode, using directory entries.
//   This does not require storing path strings during cd; it is computed on demand.
pub fn handle_argv(_argv: &[&str], context: &mut Context) {
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("/");
            return;
        }
    };

    // Resolve current directory and root inode IDs.
    let cur_id = match fs.resolve_path(".") {
        Ok(id) => id,
        Err(_) => {
            eprintln!("/");
            return;
        }
    };
    let root_id = match fs.resolve_path("/") {
        Ok(id) => id,
        Err(_) => {
            eprintln!("/");
            return;
        }
    };

    if cur_id == root_id {
        eprintln!("/");
        return;
    }

    let mut visited = HashSet::new();
    match find_path_from_root(fs, root_id, cur_id, &mut visited) {
        Ok(Some(components)) => {
            let path = format!("/{}", components.join("/"));
            println!("{}", path);
        }
        _ => {
            // Fallback if not found or error occurred
            println!("/");
        }
    }
}

// Depth-first search from root directory to target inode, returning the sequence of names.
fn find_path_from_root(
    fs: &mut FileSystem,
    root_id: u32,
    target_id: u32,
    visited: &mut HashSet<u32>,
) -> io::Result<Option<Vec<String>>> {
    dfs_dir(fs, root_id, target_id, visited)
}

fn dfs_dir(
    fs: &mut FileSystem,
    dir_id: u32,
    target_id: u32,
    visited: &mut HashSet<u32>,
) -> io::Result<Option<Vec<String>>> {
    if !visited.insert(dir_id) {
        return Ok(None);
    }

    let dir_inode = fs.read_inode(dir_id)?;
    if dir_inode.file_type != 1 {
        return Ok(None);
    }

    let mut buf = vec![0u8; DIR_ENTRY_SIZE];
    let slots = (dir_inode.file_size as usize) / DIR_ENTRY_SIZE;

    for i in 0..slots {
        fs.read_file_range(&dir_inode, (i * DIR_ENTRY_SIZE) as u64, &mut buf)?;
        let entry = DirectoryEntry::deserialize(&buf);
        if entry.is_unused() {
            continue;
        }

        let child_id = entry.inode_id;
        let name = entry.name_str().to_string();

        if child_id == target_id {
            return Ok(Some(vec![name]));
        }

        // Recurse into subdirectories only
        let child_inode: Inode = fs.read_inode(child_id)?;
        if child_inode.file_type == 1 {
            if let Some(mut tail) = dfs_dir(fs, child_id, target_id, visited)? {
                let mut result = Vec::with_capacity(tail.len() + 1);
                result.push(name);
                result.append(&mut tail);
                return Ok(Some(result));
            }
        }
    }

    Ok(None)
}
