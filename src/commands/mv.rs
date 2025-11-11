use crate::context::Context;

fn basename(path: &str) -> &str {
    path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(path)
}

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate args: need exactly two arguments: mv s1 s2
    if argv.len() != 2 {
        println!("PATH NOT FOUND");
        return;
    }
    let src_path = argv[0];
    let dst_path = argv[1];

    // Get FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve source inode and its parent + name
    let src_inode_id = match fs.resolve_path(src_path) {
        Ok(id) => id,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };
    let (src_parent_id, src_name) = match fs.resolve_parent_and_name(src_path) {
        Ok(v) => v,
        Err(_) => {
            // Parent of source path not found – treat as PATH NOT FOUND
            println!("PATH NOT FOUND");
            return;
        }
    };
    let src_parent_inode = match fs.read_inode(src_parent_id) {
        Ok(i) => i,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Decide destination parent and final name
    let (dst_parent_id, dst_name) = match fs.resolve_path(dst_path) {
        // Destination exists
        Ok(dst_existing_id) => {
            // If destination is a directory, move into it with basename(src)
            match fs.read_inode(dst_existing_id) {
                Ok(inode) => {
                    if inode.file_type == 1 {
                        // Dir: move into dir, name = basename(src)
                        let name = basename(src_path).to_string();
                        (dst_existing_id, name)
                    } else {
                        // Exists and not a directory -> treat as replace of this entry
                        // Need its parent and exact last component as new name
                        match fs.resolve_parent_and_name(dst_path) {
                            Ok((p, name)) => (p, name),
                            Err(_) => {
                                println!("PATH NOT FOUND");
                                return;
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("PATH NOT FOUND");
                    return;
                }
            }
        }
        // Destination does not exist – use its parent and last component as new name
        Err(_) => match fs.resolve_parent_and_name(dst_path) {
            Ok(v) => v,
            Err(_) => {
                println!("PATH NOT FOUND");
                return;
            }
        },
    };

    // Load destination parent inode
    let mut dst_parent_inode = match fs.read_inode(dst_parent_id) {
        Ok(i) => i,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };
    if dst_parent_inode.file_type != 1 {
        println!("PATH NOT FOUND");
        return;
    }

    // If destination entry exists, handle collision
    match fs.dir_find(&dst_parent_inode, &dst_name) {
        Ok(Some((_slot, existing_entry))) => {
            // If existing is a directory, ensure empty; else we can replace
            match fs.read_inode(existing_entry.inode_id) {
                Ok(existing_inode) => {
                    if existing_inode.file_type == 1 {
                        // directory: only allow replace if empty
                        match fs.dir_is_empty(&existing_inode) {
                            Ok(true) => {
                                // Remove the empty directory entry and free inode
                                if let Err(_) =
                                    fs.dir_remove_entry(&mut dst_parent_inode, &dst_name)
                                {
                                    println!("PATH NOT FOUND");
                                    return;
                                }
                                if let Err(_) = fs.free_inode(existing_entry.inode_id) {
                                    println!("PATH NOT FOUND");
                                    return;
                                }
                            }
                            _ => {
                                // Not empty or error -> cannot replace
                                println!("PATH NOT FOUND");
                                return;
                            }
                        }
                    } else {
                        // file or symlink: remove entry and free inode
                        if let Err(_) = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name) {
                            println!("PATH NOT FOUND");
                            return;
                        }
                        if let Err(_) = fs.free_inode(existing_entry.inode_id) {
                            println!("PATH NOT FOUND");
                            return;
                        }
                    }
                }
                Err(_) => {
                    println!("PATH NOT FOUND");
                    return;
                }
            }
        }
        Ok(None) => {
            // No collision; proceed
        }
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    }

    // Add new entry in destination parent that points to the source inode
    if let Err(_) = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, src_inode_id) {
        println!("PATH NOT FOUND");
        return;
    }

    // Remove the source entry from its parent
    let mut src_parent_inode_mut = src_parent_inode;
    if let Err(_) = fs.dir_remove_entry(&mut src_parent_inode_mut, &src_name) {
        // Best-effort rollback is complex; report failure
        println!("PATH NOT FOUND");
        return;
    }

    println!("OK");
}
