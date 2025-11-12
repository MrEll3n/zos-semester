use crate::context::Context;
use crate::fs::consts::DIR_NAME_LEN;
use crate::fs::layout::Inode;
use std::fs::File;
use std::io::Read;

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate arguments: incp <host_src> <fs_dest>
    if argv.len() != 2 {
        eprintln!("PATH NOT FOUND");
        return;
    }
    let host_src = argv[0];
    let fs_dest = argv[1];

    // Open host source file
    let mut src_file = match File::open(host_src) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Get FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve destination: support directory targets (".", existing dir, or path ending with '/')
    // If destination is a directory, use the source file's basename as the new entry name.
    let src_base = std::path::Path::new(host_src)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(host_src);

    let (parent_id, name) = {
        let treat_as_dir = fs_dest == "."
            || fs_dest.ends_with('/')
            || fs
                .resolve_path(fs_dest)
                .map(|id| {
                    fs.read_inode(id)
                        .map(|inode| inode.file_type == 1)
                        .unwrap_or(false)
                })
                .unwrap_or(false);

        if treat_as_dir {
            // Determine the directory path (strip trailing '/' if present)
            let dir_path = if fs_dest == "." {
                "."
            } else if fs_dest.ends_with('/') {
                fs_dest.trim_end_matches('/')
            } else {
                fs_dest
            };
            let dir_id = match fs.resolve_path(dir_path) {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("PATH NOT FOUND");
                    return;
                }
            };
            (dir_id, src_base.to_string())
        } else {
            match fs.resolve_parent_and_name(fs_dest) {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("PATH NOT FOUND");
                    return;
                }
            }
        }
    };

    // Validate name: empty -> PATH NOT FOUND, too long -> NAME TOO LONG

    if name.is_empty() || name == "." || name == ".." {
        eprintln!("PATH NOT FOUND");

        return;
    }

    if name.len() > DIR_NAME_LEN {
        eprintln!("NAME TOO LONG");

        return;
    }

    // Load parent inode and check collision
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };
    match fs.dir_find(&parent_inode, &name) {
        Ok(Some(_)) => {
            // Destination already exists -> per assignment for incp we don't overwrite
            eprintln!("PATH NOT FOUND");
            return;
        }
        Ok(None) => {}
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    }

    // Allocate a new inode for the destination file
    let inode_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        Ok(None) => {
            // No free inode available
            eprintln!("PATH NOT FOUND");
            return;
        }
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Initialize inode as a regular file
    let mut inode = Inode {
        file_size: 0,
        id: inode_id,

        single_directs: [0; 5],

        single_indirect: 0,
        double_indirect: 0,

        file_type: 0, // 0 = file
        link_count: 1,
        _reserved: [0; 6],
    };

    if let Err(_) = fs.write_inode(inode_id, &inode) {
        // Failed to initialize inode; free it
        let _ = fs.free_inode(inode_id);
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Copy data from host file into FS
    let mut buf = vec![0u8; 64 * 1024];
    let mut offset: u64 = 0;
    loop {
        let n = match src_file.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(_) => {
                let _ = fs.free_inode(inode_id);
                eprintln!("FILE NOT FOUND");
                return;
            }
        };

        if let Err(_) = fs.write_file_range(&mut inode, offset, &buf[..n]) {
            let _ = fs.free_inode(inode_id);
            eprintln!("PATH NOT FOUND");
            return;
        }
        offset += n as u64;
    }

    // Add directory entry in parent
    if let Err(_) = fs.dir_add_entry(&mut parent_inode, &name, inode_id) {
        let _ = fs.free_inode(inode_id);
        eprintln!("PATH NOT FOUND");
        return;
    }

    eprintln!("OK");
}
