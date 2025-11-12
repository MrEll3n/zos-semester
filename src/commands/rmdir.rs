use crate::context::Context;

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Expect exactly one argument: path to directory to remove
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

    // Resolve the target path to inode id (it must exist and be a directory)
    let dir_id = match fs.resolve_path(path) {
        Ok(id) => id,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Read inode and verify it's a directory
    let dir_inode = match fs.read_inode(dir_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    if dir_inode.file_type != 1 {
        // Not a directory
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Directory must be empty
    match fs.dir_is_empty(&dir_inode) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("NOT EMPTY");
            return;
        }
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    }

    // Resolve parent and entry name to remove the directory entry
    let (parent_id, name) = match fs.resolve_parent_and_name(path) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Load parent inode and remove the entry
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    if let Err(_) = fs.dir_remove_entry(&mut parent_inode, &name) {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Free the directory inode itself
    if let Err(_) = fs.free_inode(dir_id) {
        // If freeing failed, we have already removed the entry; but per spec just report not found
        eprintln!("FILE NOT FOUND");
        return;
    }

    eprintln!("OK");
}
