use crate::context::Context;

/// Copy command: `cp s1 s2`
///
/// Required outputs per specification:
///   OK
///   FILE NOT FOUND      (source file missing or source is a directory / unsupported)
///   PATH NOT FOUND      (destination parent path missing or filesystem not open)
///
/// Behavior:
/// - Copies regular file contents (not directories).
/// - If destination file exists and is a regular file, it is overwritten (truncated then written).
/// - If destination exists and is a directory, prints PATH NOT FOUND (treat as invalid target).
/// - Symlink source is resolved by `resolve_path` (already dereferences symlinks).
/// - Destination last component is not dereferenced (created/overwritten directly).
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Must have filesystem open.
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Validate args count.
    if argv.len() != 2 {
        println!("PATH NOT FOUND");
        return;
    }
    let src_path = argv[0];
    let dst_path = argv[1];

    // Resolve source inode.
    let src_inode_id = match fs.resolve_path(src_path) {
        Ok(id) => id,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    let src_inode = match fs.read_inode(src_inode_id) {
        Ok(inode) => inode,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Reject directories as sources (spec only talks about files).
    if src_inode.file_type != 0 {
        println!("FILE NOT FOUND");
        return;
    }

    // Read entire source content (direct-only implementation; size limited).
    let src_size = src_inode.file_size as usize;
    let mut data = vec![0u8; src_size];
    if src_size > 0 {
        if let Err(_) = fs.read_file_range(&src_inode, 0, &mut data) {
            println!("FILE NOT FOUND");
            return;
        }
    }

    // Resolve destination parent and name.
    let (parent_id, dst_name) = match fs.resolve_parent_and_name(dst_path) {
        Ok(v) => v,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Load parent inode and verify it's a directory.
    let parent_inode = match fs.read_inode(parent_id) {
        Ok(inode) => inode,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };
    if parent_inode.file_type != 1 {
        println!("PATH NOT FOUND");
        return;
    }

    // Check if destination entry already exists.
    let existing = match fs.dir_find(&parent_inode, &dst_name) {
        Ok(opt) => opt,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    let dst_inode_id = if let Some((_slot, entry)) = existing {
        // Overwrite existing file if it's a regular file.
        let mut dst_inode = match fs.read_inode(entry.inode_id) {
            Ok(inode) => inode,
            Err(_) => {
                println!("PATH NOT FOUND");
                return;
            }
        };
        if dst_inode.file_type != 0 {
            // Existing is not a regular file -> treat as invalid target path.
            println!("PATH NOT FOUND");
            return;
        }
        // Inline truncate-to-zero (free direct blocks, reset size)
        for b in dst_inode.single_directs.iter_mut() {
            if *b != 0 {
                let _ = fs.free_block(*b); // best-effort ignore errors
                *b = 0;
            }
        }
        dst_inode.file_size = 0;
        // Persist inode state after truncate
        if let Err(_) = fs.write_inode(dst_inode.id, &dst_inode) {
            println!("PATH NOT FOUND");
            return;
        }
        if !data.is_empty() {
            if let Err(_) = fs.write_file_range(&mut dst_inode, 0, &data) {
                // Could be size limit (exceeds direct pointers) or I/O error.
                println!("FILE NOT FOUND");
                return;
            }
        }
        entry.inode_id
    } else {
        // Need to create a new inode.
        let new_id = match fs.alloc_inode() {
            Ok(Some(id)) => id,
            Ok(None) => {
                // Out of inodes -> treat as file not found scenario.
                println!("FILE NOT FOUND");
                return;
            }
            Err(_) => {
                println!("PATH NOT FOUND");
                return;
            }
        };

        // Initialize new inode (reuse existing read/write)
        let mut new_inode = match fs.read_inode(new_id) {
            Ok(inode) => inode,
            Err(_) => {
                println!("PATH NOT FOUND");
                return;
            }
        };
        new_inode.file_type = 0; // regular file
        new_inode.link_count = 1;
        new_inode.file_size = 0;
        new_inode.single_directs = [0u32; 5];
        new_inode.double_indirect = 0;
        new_inode.triple_indirect = 0;

        if let Err(_) = fs.write_inode(new_id, &new_inode) {
            println!("PATH NOT FOUND");
            return;
        }

        // Write data (if any)
        if !data.is_empty() {
            if let Err(_) = fs.write_file_range(&mut new_inode, 0, &data) {
                // Rollback inode to free (best-effort)
                let _ = fs.free_inode(new_id);
                println!("FILE NOT FOUND");
                return;
            }
        }

        // Add directory entry
        let mut parent_mut = match fs.read_inode(parent_id) {
            Ok(inode) => inode,
            Err(_) => {
                println!("PATH NOT FOUND");
                return;
            }
        };
        if let Err(_) = fs.dir_add_entry(&mut parent_mut, &dst_name, new_id) {
            // Rollback inode
            let _ = fs.free_inode(new_id);
            println!("PATH NOT FOUND");
            return;
        }

        new_id
    };

    // At this point copy succeeded.
    let _ = dst_inode_id; // suppress unused warning if not used further.
    println!("OK");
}
