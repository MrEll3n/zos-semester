use crate::fs::consts::*;

#[repr(C)]
pub struct Superblock {
    // fs metadata
    pub magic: [u8; 4], // 4 B
    pub version: u16,   // 2 B
    // blocks
    pub block_size: u16,   // 2 B
    pub total_blocks: u32, // 4 B
    // bitmap
    pub bitmap_start: u32,  // 4 B
    pub bitmap_blocks: u32, // 4 B
    // inode table
    pub inode_table_start: u32,  // 4 B
    pub inode_table_blocks: u32, // 4 B
    pub inode_count: u32,        // 4 B
    //data
    pub data_start: u32,    // 4 B
    pub root_inode_id: u32, // 4 B
}

// impl Default for Superblock {
//     fn default() -> Self {
//         Self {
//             magic: FS_MAGIC,
//             version: FS_VERSION,
//             block_size: DEFAULT_BLOCK_SIZE,
//             total_blocks: ,
//             bitmap_start: u32,
//             bitmap_blocks: u32,
//             inode_table_start: u32,
//             inode_table_blocks: u32,
//             inode_count: DEFAULT_INODE_COUNT,
//             data_start: u32,
//             root_inode_id: u32,
//         }
//     }
// }
