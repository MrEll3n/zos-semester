pub const FS_MAGIC: [u8; 4] = *b"ELFS";
pub const VERSION: u16 = 1;
pub const DEFAULT_BLOCK_SIZE: u32 = 4096;

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
