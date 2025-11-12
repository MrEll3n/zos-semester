use crate::context::Context;

/// rmslink <path>
/// Odstraní symbolický link (symlink) na dané cestě BEZ dereference cíle.
///
/// Výstupy:
///   OK
///   FILE NOT FOUND  (neexistuje, není symlink, rodič neexistuje / není dir, FS není otevřený)
///
/// Chování:
/// - Najde rodičovský adresář a jméno poslední komponenty (nepoužívá resolve_path, aby nedošlo k dereferenci).
/// - V adresáři vyhledá položku se jménem.
/// - Ověří, že inode má file_type == 2 (symlink).
/// - Odebere položku z adresáře a uvolní inode.
/// - Pokud jméno odkazuje na běžný soubor nebo adresář, nic nemaže (FILE NOT FOUND).
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Ověření argumentů
    if argv.len() != 1 {
        eprintln!("FILE NOT FOUND");
        return;
    }
    let path = argv[0];

    // Získání FS instance
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Získat parent + name (bez dereference symlinku na konci)
    let (parent_id, name) = match fs.resolve_parent_and_name(path) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    if name.is_empty() {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Načíst parent inode
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    if parent_inode.file_type != 1 {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Najít položku v adresáři
    let entry = match fs.dir_find(&parent_inode, &name) {
        Ok(Some((_slot, e))) => e,
        _ => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Načíst inode cíle
    let target_inode = match fs.read_inode(entry.inode_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Musí být symlink
    if target_inode.file_type != 2 {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Odebrat položku z adresáře
    if let Err(_) = fs.dir_remove_entry(&mut parent_inode, &name) {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Uvolnit inode symlinku
    if let Err(_) = fs.free_inode(entry.inode_id) {
        eprintln!("FILE NOT FOUND");
        return;
    }

    eprintln!("OK");
}
