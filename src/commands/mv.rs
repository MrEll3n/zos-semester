use crate::context::Context;

fn basename(path: &str) -> &str {
    path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(path)
}

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate args: need exactly two arguments: mv s1 s2

    if argv.len() != 2 {
        eprintln!("PATH NOT FOUND");

        return;
    }

    let src_path = argv[0];

    let dst_path = argv[1];

    // Get FS

    let fs = match context.fs_mut() {
        Ok(fs) => fs,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    // Resolve source inode and its parent + name

    let src_inode_id = match fs.resolve_path(src_path) {
        Ok(id) => id,

        Err(_) => {
            eprintln!("FILE NOT FOUND");

            return;
        }
    };

    let (src_parent_id, src_name) = match fs.resolve_parent_and_name(src_path) {
        Ok(v) => v,

        Err(_) => {
            // Parent of source path not found – treat as PATH NOT FOUND

            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    let src_parent_inode = match fs.read_inode(src_parent_id) {
        Ok(i) => i,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    // Decide destination parent and final name

    let (dst_parent_id, dst_name) = match fs.resolve_path(dst_path) {
        // Destination exists
        Ok(dst_existing_id) => {
            // If destination resolves to the same inode as source, no-op

            if dst_existing_id == src_inode_id {
                eprintln!("OK");

                return;
            }

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
                                eprintln!("PATH NOT FOUND");

                                return;
                            }
                        }
                    }
                }

                Err(_) => {
                    eprintln!("PATH NOT FOUND");

                    return;
                }
            }
        }

        // Destination does not exist – use its parent and last component as new name
        Err(_) => match fs.resolve_parent_and_name(dst_path) {
            Ok(v) => v,

            Err(_) => {
                eprintln!("PATH NOT FOUND");

                return;
            }
        },
    };

    // Disallow special names as destination
    if dst_name == "." || dst_name == ".." {
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Load destination parent inode

    let mut dst_parent_inode = match fs.read_inode(dst_parent_id) {
        Ok(i) => i,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    if dst_parent_inode.file_type != 1 {
        eprintln!("PATH NOT FOUND");

        return;
    }

    // No-op: moving to the same parent and same name

    if dst_parent_id == src_parent_id && dst_name == src_name {
        eprintln!("OK");

        return;
    }

    // If destination entry exists, handle collision (safe / rollback-aware)

    match fs.dir_find(&dst_parent_inode, &dst_name) {
        Ok(Some((_slot, existing_entry))) => {
            // Case 1: Destination entry already points to source inode (self-target)

            if existing_entry.inode_id == src_inode_id {
                // If parent differs, we are effectively creating a second hard link then removing the old one;

                // current FS semantics: we only allow single link (link_count not leveraged here),

                // so treat it as moving across dirs: just remove old source entry.

                if src_parent_id != dst_parent_id {
                    let mut src_parent_inode_mut = match fs.read_inode(src_parent_id) {
                        Ok(i) => i,

                        Err(_) => {
                            eprintln!("PATH NOT FOUND");

                            return;
                        }
                    };

                    if let Err(_) = fs.dir_remove_entry(&mut src_parent_inode_mut, &src_name) {
                        eprintln!("PATH NOT FOUND");

                        return;
                    }
                }

                eprintln!("OK");

                return;
            }

            // Load existing target inode

            let existing_inode = match fs.read_inode(existing_entry.inode_id) {
                Ok(i) => i,

                Err(_) => {
                    eprintln!("PATH NOT FOUND");

                    return;
                }
            };

            // Directories can be replaced only if empty

            if existing_inode.file_type == 1 {
                match fs.dir_is_empty(&existing_inode) {
                    Ok(true) => { /* allowed */ }

                    _ => {
                        eprintln!("PATH NOT FOUND");

                        return;
                    }
                }
            }

            // Stage removal: remove target directory entry but DO NOT free inode yet (to allow rollback)

            if let Err(_) = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name) {
                eprintln!("PATH NOT FOUND");

                return;
            }

            // Try to insert new destination entry pointing to the source inode

            if let Err(_) = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, src_inode_id) {
                // Rollback attempt: re-add old entry with original inode id

                let _ = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, existing_entry.inode_id);

                eprintln!("PATH NOT FOUND");

                return;
            }

            // New entry added; now remove source entry

            let mut src_parent_inode_mut = match fs.read_inode(src_parent_id) {
                Ok(i) => i,

                Err(_) => {
                    eprintln!("PATH NOT FOUND");

                    return;
                }
            };

            if let Err(_) = fs.dir_remove_entry(&mut src_parent_inode_mut, &src_name) {
                // Rollback attempt: remove newly added entry and restore old one

                let _ = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name);

                let _ = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, existing_entry.inode_id);

                eprintln!("PATH NOT FOUND");

                return;
            }

            // Finally free the replaced inode (file or empty dir)

            if let Err(_) = fs.free_inode(existing_inode.id) {
                // Non-fatal: we already performed the move, but report generic error per spec style

                eprintln!("PATH NOT FOUND");

                return;
            }
        }

        Ok(None) => {

            // No collision; proceed normally
        }

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    }

    // Add new entry in destination parent that points to the source inode

    if let Err(_) = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, src_inode_id) {
        eprintln!("PATH NOT FOUND");

        return;
    }

    // Remove the source entry from its parent

    let mut src_parent_inode_mut = match fs.read_inode(src_parent_id) {
        Ok(i) => i,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    if let Err(_) = fs.dir_remove_entry(&mut src_parent_inode_mut, &src_name) {
        // Best-effort rollback is complex; report failure

        eprintln!("PATH NOT FOUND");

        return;
    }

    eprintln!("OK");
}
