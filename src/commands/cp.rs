use crate::context::Context;

/// Safe copy command: `cp <src> <dst>`
///
/// Outputs per specification:
///   OK
///   FILE NOT FOUND
///   PATH NOT FOUND
///   NAME TOO LONG
///
/// Úprava: cílový path nesmí být adresář (ani ".", "..", ani cokoliv co rezolvuje na dir).
/// Kopírování do adresáře se odmítá dle požadavku.
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Otevři FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    if argv.len() != 2 {
        eprintln!("PATH NOT FOUND");
        return;
    }

    let src_path = argv[0];
    let dst_path = argv[1];

    // Resolve source
    let src_inode_id = match fs.resolve_path(src_path) {
        Ok(id) => id,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    let src_inode = match fs.read_inode(src_inode_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    if src_inode.file_type != 0 {
        eprintln!("FILE NOT FOUND");
        return;
    }
    debug_assert_eq!(src_inode.id, src_inode_id);
    debug_assert_eq!(src_inode.file_type, 0);

    // Načti obsah zdroje
    let src_size = src_inode.file_size as usize;
    let mut data = vec![0u8; src_size];
    if src_size > 0 {
        if let Err(_) = fs.read_file_range(&src_inode, 0, &mut data) {
            eprintln!("FILE NOT FOUND");
            return;
        }
    }

    // Odmítnout cíle, které jsou adresáře nebo speciální komponenty
    if dst_path == "." || dst_path == ".." || dst_path.ends_with('/') {
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Pokud path už existuje a je adresář, odmítnout
    if fs
        .resolve_path(dst_path)
        .map(|id| {
            fs.read_inode(id)
                .map(|ino| ino.file_type == 1)
                .unwrap_or(false)
        })
        .unwrap_or(false)
    {
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Získat parent + jméno
    let (dst_parent_id, dst_name) = match fs.resolve_parent_and_name(dst_path) {
        Ok((pid, name)) => (pid, name),
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Validace jména
    if dst_name.is_empty() || dst_name == "." || dst_name == ".." || dst_name.contains('/') {
        eprintln!("PATH NOT FOUND");
        return;
    }
    if dst_name.len() > crate::fs::consts::DIR_NAME_LEN {
        eprintln!("NAME TOO LONG");
        return;
    }

    // Načti parent inode
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
    debug_assert_eq!(dst_parent_inode.id, dst_parent_id);
    debug_assert_eq!(dst_parent_inode.file_type, 1);

    // Kolize? – existuje položka se jménem v parentu
    let existing = match fs.dir_find(&dst_parent_inode, &dst_name) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // No-op: cp file file -> pokud jde o stejný inode
    if let Some((_slot, entry)) = &existing {
        if entry.inode_id == src_inode_id {
            eprintln!("OK");
            return;
        }
    }

    // Overwrite?
    if let Some((_slot, entry)) = existing {
        let old_inode = match fs.read_inode(entry.inode_id) {
            Ok(i) => i,
            Err(_) => {
                eprintln!("PATH NOT FOUND");
                return;
            }
        };
        if old_inode.file_type != 0 {
            eprintln!("PATH NOT FOUND");
            return;
        }

        // --- nový inode ---
        let new_id = match fs.alloc_inode() {
            Ok(Some(id)) => id,
            Ok(None) => {
                eprintln!("FILE NOT FOUND");
                return;
            }
            Err(_) => {
                eprintln!("PATH NOT FOUND");
                return;
            }
        };
        let mut new_inode = match fs.read_inode(new_id) {
            Ok(i) => i,
            Err(_) => {
                let _ = fs.free_inode(new_id);
                eprintln!("PATH NOT FOUND");
                return;
            }
        };
        new_inode.id = new_id;
        new_inode.file_type = 0;
        new_inode.link_count = 1;
        new_inode.file_size = 0;
        new_inode.single_directs = [0; 5];
        new_inode.single_indirect = 0;
        new_inode.double_indirect = 0;

        if let Err(_) = fs.write_inode(new_id, &new_inode) {
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }

        // Zápis dat
        if !data.is_empty() {
            debug_assert_eq!(new_inode.file_type, 0);
            if let Err(_) = fs.write_file_range(&mut new_inode, 0, &data) {
                let _ = fs.free_inode(new_id);
                eprintln!("FILE NOT FOUND");
                return;
            }
            if let Err(_) = fs.write_inode(new_id, &new_inode) {
                let _ = fs.free_inode(new_id);
                eprintln!("PATH NOT FOUND");
                return;
            }
        }

        // Nahraď dirent
        if let Err(_) = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name) {
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }
        if let Err(_) = fs.write_inode(dst_parent_id, &dst_parent_inode) {
            let _ = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, old_inode.id);
            let _ = fs.write_inode(dst_parent_id, &dst_parent_inode);
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }

        if let Err(_) = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, new_id) {
            let _ = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, old_inode.id);
            let _ = fs.write_inode(dst_parent_id, &dst_parent_inode);
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }
        if let Err(_) = fs.write_inode(dst_parent_id, &dst_parent_inode) {
            let _ = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name);
            let _ = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, old_inode.id);
            let _ = fs.write_inode(dst_parent_id, &dst_parent_inode);
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }

        let _ = fs.free_inode(old_inode.id);
        eprintln!("OK");
        return;
    }

    // --- cílový soubor neexistuje ---
    let new_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        Ok(None) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };
    let mut new_inode = match fs.read_inode(new_id) {
        Ok(i) => i,
        Err(_) => {
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }
    };
    new_inode.id = new_id;
    new_inode.file_type = 0;
    new_inode.link_count = 1;
    new_inode.file_size = 0;
    new_inode.single_directs = [0; 5];
    new_inode.single_indirect = 0;
    new_inode.double_indirect = 0;

    if let Err(_) = fs.write_inode(new_id, &new_inode) {
        let _ = fs.free_inode(new_id);
        eprintln!("PATH NOT FOUND");
        return;
    }

    if !data.is_empty() {
        debug_assert_eq!(new_inode.file_type, 0);
        if let Err(_) = fs.write_file_range(&mut new_inode, 0, &data) {
            let _ = fs.free_inode(new_id);
            eprintln!("FILE NOT FOUND");
            return;
        }
        if let Err(_) = fs.write_inode(new_id, &new_inode) {
            let _ = fs.free_inode(new_id);
            eprintln!("PATH NOT FOUND");
            return;
        }
    }

    if let Err(_) = fs.dir_add_entry(&mut dst_parent_inode, &dst_name, new_id) {
        let _ = fs.free_inode(new_id);
        eprintln!("PATH NOT FOUND");
        return;
    }
    if let Err(_) = fs.write_inode(dst_parent_id, &dst_parent_inode) {
        let _ = fs.dir_remove_entry(&mut dst_parent_inode, &dst_name);
        let _ = fs.write_inode(dst_parent_id, &dst_parent_inode);
        let _ = fs.free_inode(new_id);
        eprintln!("PATH NOT FOUND");
        return;
    }

    eprintln!("OK");
}
