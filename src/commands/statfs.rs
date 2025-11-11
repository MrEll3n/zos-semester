use crate::context::Context;
use crate::fs::consts::BLOCK_SIZE;
use crate::fs::io::{bitmap_is_set, load_bitmap, read_inode, read_superblock};
use std::fs::OpenOptions;

/// statfs command:
/// Prints basic filesystem statistics:
/// - total FS size
/// - block size
/// - data blocks: total, used, free
/// - inodes: total, used, free
/// - number of directories
///
/// Errors:
/// - prints "PATH NOT FOUND" if filesystem is not opened in the context
/// - prints "FILE NOT FOUND" if the underlying image cannot be opened or superblock can't be read
pub fn handle_argv(_argv: &[&str], context: &mut Context) {
    // Ensure filesystem path is available
    let fs_path = match context.fs_path() {
        Some(p) => p,
        None => {
            println!("PATH NOT FOUND");
            return;
        }
    };

    // Open image file read-only for stats
    let mut f = match OpenOptions::new().read(true).open(fs_path) {
        Ok(f) => f,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Read superblock
    let sb = match read_superblock(&mut f) {
        Ok(sb) => sb,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Load data block bitmap to compute used/free data blocks
    let bitmap = match load_bitmap(&mut f, &sb) {
        Ok(b) => b,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Count used data blocks from bitmap
    let mut used_blocks: u32 = 0;
    for idx in 0..sb.block_count {
        if bitmap_is_set(&bitmap, idx) {
            used_blocks += 1;
        }
    }
    let free_blocks = sb.block_count.saturating_sub(used_blocks);

    // Count inodes usage and directories by scanning the inode table
    let mut used_inodes: u32 = 0;
    let mut dirs: u32 = 0;
    for inode_id in 0..sb.inode_count {
        match read_inode(&mut f, &sb, inode_id) {
            Ok(inode) => {
                if inode.link_count != 0 {
                    used_inodes += 1;
                    if inode.file_type == 1 {
                        dirs += 1;
                    }
                }
            }
            Err(_) => {
                // Should not happen within 0..inode_count; ignore if it does
                continue;
            }
        }
    }
    let free_inodes = sb.inode_count.saturating_sub(used_inodes);

    // Print statistics
    println!("File system size: {} B", sb.fs_size);
    println!("Block size: {} B", BLOCK_SIZE);
    println!(
        "Data blocks: all={} used={} free={}",
        sb.block_count, used_blocks, free_blocks
    );
    println!(
        "I-nodes: all={} used={} free={}",
        sb.inode_count, used_inodes, free_inodes
    );
    println!("Directories: {}", dirs);
}
