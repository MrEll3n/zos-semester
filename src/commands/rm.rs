use crate::context::Context;

/// Standalone `rm` command handler.
///
/// Spec:
/// - rm s1 -> OK | FILE NOT FOUND
///
/// Chování:
/// - Cíl musí být běžný soubor (ne adresář).
/// - Při chybě (FS neotevřen, neexistuje, je to adresář, chyba při odstranění)
///   vytiskne "FILE NOT FOUND".
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

    // Najít cílový inode
    let inode_id = match fs.resolve_path(path) {
        Ok(id) => id,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Načíst inode a ověřit, že není adresář
    let inode = match fs.read_inode(inode_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };
    if inode.file_type == 1 {
        // Je to adresář -> rm neumí mazat adresáře
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Získat rodiče a jméno poslední komponenty
    let (parent_id, name) = match fs.resolve_parent_and_name(path) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Načíst inode rodiče
    let mut parent_inode = match fs.read_inode(parent_id) {
        Ok(i) => i,
        Err(_) => {
            eprintln!("FILE NOT FOUND");
            return;
        }
    };

    // Odebrat záznam z adresáře rodiče
    if let Err(_) = fs.dir_remove_entry(&mut parent_inode, &name) {
        eprintln!("FILE NOT FOUND");
        return;
    }

    // Uvolnit inode cílového souboru (uvolní bloky a označí inode jako volný)
    if let Err(_) = fs.free_inode(inode_id) {
        // V extrémním případě by bylo vhodné pokusit se zvrátit odstranění záznamu,
        // ale v rámci zadání stačí signalizovat chybu.
        eprintln!("FILE NOT FOUND");
        return;
    }

    eprintln!("OK");
}
