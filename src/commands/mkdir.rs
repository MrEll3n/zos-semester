use crate::context::Context;
use crate::fs::layout::Inode;

/// mkdir a1
/// Outputs: OK | PATH NOT FOUND | EXIST
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate args
    if argv.len() != 1 {
        eprintln!("PATH NOT FOUND");
        return;
    }
    let target_path = argv[0];

    // Get FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve parent and name
    let (parent_id, name) = match fs.resolve_parent_and_name(target_path) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Validate directory name constraints then load parent inode and ensure it's a directory

    if name.is_empty() || name == "." || name == ".." {
        eprintln!("PATH NOT FOUND");
        return;
    }
    if name.len() > crate::fs::consts::DIR_NAME_LEN {
        eprintln!("NAME TOO LONG");
        return;
    }

    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    if parent_inode.file_type != 1 {
        eprintln!("PATH NOT FOUND");

        return;
    }

    // Check existence
    match fs.dir_find(&parent_inode, &name) {
        Ok(Some(_)) => {
            eprintln!("EXIST");
            return;
        }
        Ok(None) => {}
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    }

    // Allocate new inode
    let new_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        _ => {
            // No free inode (or IO error) -> map to PATH NOT FOUND as per simplified outputs
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    let inode = Inode {
        file_size: 0,

        id: new_id,

        single_directs: [0; 5],

        single_indirect: 0,

        double_indirect: 0,

        file_type: 1, // dir
        link_count: 1,

        _reserved: [0; 6],
    };

    // Persist inode
    if fs.write_inode(new_id, &inode).is_err() {
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Add directory entry to parent
    if let Err(_) = fs.dir_add_entry(&mut parent_inode, &name, new_id) {
        // Best-effort cleanup
        let _ = fs.free_inode(new_id);
        eprintln!("PATH NOT FOUND");
        return;
    }

    eprintln!("OK");
}
