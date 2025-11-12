use crate::context::Context;
use crate::fs::consts::DIR_NAME_LEN;

/// slink s1 s2
/// Vytvoří symbolický link s názvem s2, který odkazuje na s1 (uložené jako textový obsah).
/// Výstupy:
/// - "OK"
/// - "EXIST" (pokud položka s daným jménem v cílovém adresáři existuje)
/// - "PATH NOT FOUND" (neexistující cesta, neplatné jméno, nebo FS není otevřený)
/// - "CANNOT CREATE FILE" (není místo na inode/bloky, nebo jiná chyba zápisu)
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Ověření argumentů
    if argv.len() != 2 {
        eprintln!("PATH NOT FOUND");
        return;
    }
    let target_path_str = argv[0]; // symlink target (může být klidně neexistující - "dangling")
    let link_path = argv[1]; // cílová cesta pro vytvoření symlinku

    // Získání FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Rozložení cílové cesty na parent adresář a jméno entry
    let (parent_id, name) = match fs.resolve_parent_and_name(link_path) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };

    // Kontrola délky jména (max 12 B dle zadání)
    if name.is_empty() || name.len() > DIR_NAME_LEN {
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Načíst parent inode a ověřit, že jde o adresář
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    };
    if parent_inode.file_type != 1 {
        // není adresář
        eprintln!("PATH NOT FOUND");
        return;
    }

    // Ověřit kolizi jména
    match fs.dir_find(&parent_inode, &name) {
        Ok(Some(_)) => {
            eprintln!("EXIST");
            return;
        }
        Ok(None) => {} // OK, pokračujeme
        Err(_) => {
            eprintln!("PATH NOT FOUND");
            return;
        }
    }

    // Alokovat inode pro symlink
    let inode_id = match fs.alloc_inode() {
        Ok(Some(id)) => id,
        Ok(None) => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
        Err(_) => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
    };

    // Vytvořit nový inode symlinku
    let mut link_inode = match fs.read_inode(inode_id) {
        Ok(ino) => ino,
        Err(_) => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
    };
    link_inode.id = inode_id;
    link_inode.file_type = 2; // symlink
    link_inode.link_count = 1;

    link_inode.file_size = 0;

    link_inode.single_directs = [0; 5];

    link_inode.single_indirect = 0;

    link_inode.double_indirect = 0;

    // Zapsat obsah symlinku (target path jako text)
    let target_bytes = target_path_str.as_bytes();
    if let Err(_) = fs.write_file_range(&mut link_inode, 0, target_bytes) {
        // Vrátit inode do poolu (best-effort)
        let _ = fs.free_inode(inode_id);
        eprintln!("CANNOT CREATE FILE");
        return;
    }

    // Zapsat inode na disk (write_file_range už velikost nastavuje, ale flushneme pro jistotu)
    if let Err(_) = fs.write_inode(inode_id, &link_inode) {
        let _ = fs.free_inode(inode_id);
        eprintln!("CANNOT CREATE FILE");
        return;
    }

    // Zapsat položku do rodiče
    if let Err(e) = fs.dir_add_entry(&mut parent_inode, &name, inode_id) {
        // Mapování chyb na zadání
        use std::io::ErrorKind;
        let _ = fs.free_inode(inode_id);
        match e.kind() {
            ErrorKind::AlreadyExists => eprintln!("EXIST"),
            ErrorKind::InvalidInput | ErrorKind::NotFound => eprintln!("PATH NOT FOUND"),
            _ => eprintln!("CANNOT CREATE FILE"),
        }
        return;
    }

    eprintln!("OK");
}
