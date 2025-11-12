use crate::context::Context;

use crate::fs::consts::BLOCK_SIZE;

use crate::fs::io::bitmap_is_set;

/// statfs command (reworked to use in-memory FS state):
/// - Čte superblock + bitmapu z otevřeného FileSystem instance (po flush), neotevírá znovu image.
/// - Počítá využité datové bloky přímo z in-memory bitmapy.
/// - Počítá použité inody a adresáře přečtením inodové tabulky přes veřejné API.
///
/// POZNÁMKA: Kvůli tomu, že pole ve `FileSystem` jsou privátní, používáme
/// bezpečný wrapper s `unsafe` přetypováním na repliku struktury (stejné pořadí
/// polí). Je to omezení současného návrhu – ideální by bylo přidat veřejné
/// accessor metody. Tento přístup je izolovaný pouze v tomto příkazu.
pub fn handle_argv(_argv: &[&str], context: &mut Context) {
    // Otevřený FS
    let fs = match context.fs_mut() {
        Ok(fs) => fs,

        Err(_) => {
            eprintln!("PATH NOT FOUND");

            return;
        }
    };

    // Flush – zajistí, že bitmapa v paměti je zapsaná na disk (pro konzistenci i při pádu),
    // ale my dál pracujeme s in-memory kopií.
    let _ = fs.flush();

    // Shromáždění potřebných hodnot ze superbloku v omezeném scope,
    // aby se uvolnil immutable borrow před voláním fs.read_inode (mutable).
    let (fs_size, block_count, inode_count, used_blocks, free_blocks) = {
        let sb_ref = fs.superblock();
        let bitmap = fs.data_bitmap();
        let mut used_blocks_local: u32 = 0;
        for rel in 0..sb_ref.block_count {
            if bitmap_is_set(bitmap, rel) {
                used_blocks_local += 1;
            }
        }
        let free_blocks_local = sb_ref.block_count.saturating_sub(used_blocks_local);
        (
            sb_ref.fs_size,
            sb_ref.block_count,
            sb_ref.inode_count,
            used_blocks_local,
            free_blocks_local,
        )
    };

    // Počítání inodů (nyní je immutable borrow uvolněn, můžeme volat read_inode)
    let mut used_inodes: u32 = 0;
    let mut dirs: u32 = 0;
    for inode_id in 0..inode_count {
        if let Ok(inode) = fs.read_inode(inode_id) {
            if inode.link_count != 0 {
                used_inodes += 1;
                if inode.file_type == 1 {
                    dirs += 1;
                }
            }
        }
    }
    let free_inodes = inode_count.saturating_sub(used_inodes);

    eprintln!("File system size: {} B", fs_size);
    eprintln!("Block size: {} B", BLOCK_SIZE);
    eprintln!(
        "Data blocks: all={} used={} free={}",
        block_count, used_blocks, free_blocks
    );
    eprintln!(
        "I-nodes: all={} used={} free={}",
        inode_count, used_inodes, free_inodes
    );
    eprintln!("Directories: {}", dirs);
}
