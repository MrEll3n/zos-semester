use crate::context::Context;

/// info <path>

/// Prints: "NAME – SIZE B – i-node INODE_ID – soft links: COUNT"
/// On error: "FILE NOT FOUND"

///
/// Soft link count = počet symlink inode (file_type == 2, link_count > 0),
/// jejichž cílová cesta (uložená jako obsah symlinku) se aktuálně resolvuje
/// na tento inode. Dangling symlinky nebo symlinky ukazující jinam se nepočítají.
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Exactly one argument expected.
    if argv.len() != 1 {
        eprintln!("FILE NOT FOUND");

        return;
    }

    let path = argv[0];

    // Access filesystem.
    let fs = match context.fs_mut() {
        Ok(fs) => fs,

        Err(_) => {
            eprintln!("FILE NOT FOUND");

            return;
        }
    };

    // Resolve target inode id.
    let inode_id = match fs.resolve_path(path) {
        Ok(id) => id,

        Err(_) => {
            eprintln!("FILE NOT FOUND");

            return;
        }
    };

    // Read inode.
    let inode = match fs.read_inode(inode_id) {
        Ok(ino) => ino,

        Err(_) => {
            eprintln!("FILE NOT FOUND");

            return;
        }
    };

    let name = last_component_or_root(path);

    // Scan all inodes for symlinks pointing to this inode.
    let mut soft_links: u32 = 0;
    let total = fs.inode_count();
    for sid in 0..total {
        // Read symlink inode
        let sy_inode = match fs.read_inode(sid) {
            Ok(i) => i,
            Err(_) => continue,
        };
        if sy_inode.file_type != 2 || sy_inode.link_count == 0 {
            continue;
        }
        // Read its stored target path
        let target = match fs.readlink_target(sid) {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Try resolving target; if resolves to queried inode, count it
        if let Ok(tid) = fs.resolve_path(&target) {
            if tid == inode_id {
                soft_links += 1;
            }
        }
    }

    eprintln!(
        "{} – {} B – i-node {} – soft links: {}",
        name, inode.file_size, inode_id, soft_links
    );
}

fn last_component_or_root(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }

    let trimmed = path.trim_end_matches('/');

    if trimmed.is_empty() {
        return "/".to_string();
    }

    match trimmed.rsplit('/').next() {
        Some(comp) if !comp.is_empty() => comp.to_string(),

        _ => "/".to_string(),
    }
}
