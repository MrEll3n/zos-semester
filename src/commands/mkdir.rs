use crate::context::Context;
use crate::fs::layout::Inode;

/// mkdir a1
/// Outputs: OK | PATH NOT FOUND | EXIST
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate args
    if argv.len() != 1 {
        println!("PATH NOT FOUND");
        return;
    }
    let target_path = argv[0];

    // Get FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Resolve parent and name
    let (parent_id, name) = match fs.resolve_parent_and_name(target_path) {
        Ok(v) => v,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Load parent inode and ensure it's a directory
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    };
    if parent_inode.file_type != 1 {
        println!("PATH NOT FOUND");
        return;
    }

    // Check existence
    match fs.dir_find(&parent_inode, &name) {
        Ok(Some(_)) => {
            println!("EXIST");
            return;
        }
        Ok(None) => {}
        Err(_) => {
            println!("PATH NOT FOUND");
            return;
        }
    }

    // Allocate new inode
    let new_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        _ => {
            // No free inode (or IO error) -> map to PATH NOT FOUND as per simplified outputs
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Initialize directory inode
    let inode = Inode {
        file_size: 0,
        id: new_id,
        single_directs: [0; 5],
        double_indirect: 0,
        triple_indirect: 0,
        file_type: 1, // dir
        link_count: 1,
        _reserved: [0; 6],
    };

    // Persist inode
    if fs.write_inode(new_id, &inode).is_err() {
        println!("PATH NOT FOUND");
        return;
    }

    // Add directory entry to parent
    if let Err(_) = fs.dir_add_entry(&mut parent_inode, &name, new_id) {
        // Best-effort cleanup
        let _ = fs.free_inode(new_id);
        println!("PATH NOT FOUND");
        return;
    }

    println!("OK");
}
