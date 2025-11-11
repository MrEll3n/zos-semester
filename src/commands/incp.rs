use crate::context::Context;
use crate::fs::consts::DIR_NAME_LEN;
use crate::fs::layout::Inode;
use std::fs::File;
use std::io::Read;

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate arguments: incp <host_src> <fs_dest>
    if argv.len() != 2 {
        println!("PATH NOT FOUND");
        return;
    }
    let host_src = argv[0];
    let fs_dest = argv[1];

    // Open host source file
    let mut src_file = match File::open(host_src) {
        Ok(f) => f,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Get FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve parent directory and name inside FS for destination
    let (parent_id, name) = match fs.resolve_parent_and_name(fs_dest) {
        Ok(v) => v,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Enforce name length constraint (12 bytes max)
    if name.is_empty() || name.len() > DIR_NAME_LEN {
        println!("PATH NOT FOUND");
        return;
    }

    // Load parent inode and check collision
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };
    match fs.dir_find(&parent_inode, &name) {
        Ok(Some(_)) => {
            // Destination already exists -> per assignment for incp we don't overwrite
            println!("PATH NOT FOUND");
            return;
        }
        Ok(None) => {}
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    }

    // Allocate a new inode for the destination file
    let inode_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        Ok(None) => {
            // No free inode available
            println!("PATH NOT FOUND");
            return;
        }
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Initialize inode as a regular file
    let mut inode = Inode {
        file_size: 0,
        id: inode_id,
        single_directs: [0; 5],
        double_indirect: 0,
        triple_indirect: 0,
        file_type: 0, // 0 = file
        link_count: 1,
        _reserved: [0; 6],
    };

    if let Err(_) = fs.write_inode(inode_id, &inode) {
        // Failed to initialize inode; free it
        let _ = fs.free_inode(inode_id);
        println!("PATH NOT FOUND");
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
                println!("FILE NOT FOUND");
                return;
            }
        };

        if let Err(_) = fs.write_file_range(&mut inode, offset, &buf[..n]) {
            let _ = fs.free_inode(inode_id);
            println!("PATH NOT FOUND");
            return;
        }
        offset += n as u64;
    }

    // Add directory entry in parent
    if let Err(_) = fs.dir_add_entry(&mut parent_inode, &name, inode_id) {
        let _ = fs.free_inode(inode_id);
        println!("PATH NOT FOUND");
        return;
    }

    println!("OK");
}
