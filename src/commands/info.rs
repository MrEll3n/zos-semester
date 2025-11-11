use crate::context::Context;

/// info <path>
/// Prints: "NAME – SIZE B – i-node INODE_ID – odkazy LINK_COUNT"
/// On error: "FILE NOT FOUND"
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Expect exactly one argument (the path).
    if argv.len() != 1 {
        println!("FILE NOT FOUND");
        return;
    }
    let path = argv[0];

    // Access filesystem from context.
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Resolve path to inode id.
    let inode_id = match fs.resolve_path(path) {
        Ok(id) => id,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Read inode metadata.
    let inode = match fs.read_inode(inode_id) {
        Ok(ino) => ino,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Determine name (last component of the path), allowing root "/".
    let name = last_component_or_root(path);

    // Output per assignment: NAME – SIZE – i-node – odkazy
    println!(
        "{} – {} B – i-node {} – links {}",
        name, inode.file_size, inode_id, inode.link_count
    );
}

fn last_component_or_root(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }
    // Trim trailing slashes except for root.
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rsplit('/').next() {
        Some(comp) if !comp.is_empty() => comp.to_string(),
        _ => "/".to_string(),
    }
}
