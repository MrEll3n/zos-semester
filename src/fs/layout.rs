// Structure that represents File system's Superblock - 40 B
#[repr(C)]
pub struct Superblock {
    pub fs_size: u64,       // 8 B (offset 0..7)
    pub magic: [u8; 4],     // 4 B (offset 8..11)
    pub root_inode_id: u32, // 4 B (offset 12..15)
    pub bitmap_start: u32,  // 4 B
    pub bitmap_count: u32,  // 4 B
    pub block_start: u32,   // 4 B
    pub block_count: u32,   // 4 B
    pub inode_start: u32,   // 4 B
    pub inode_count: u32,   // 4 B
}

// Structure that represents one inode - 48 B
#[repr(C)]
pub struct Inode {
    pub file_size: u64,           // 8 (offset 0)
    pub id: u32,                  // 4 (offset 8)
    pub single_directs: [u32; 5], // 20 (offset 12..31)
    pub double_indirect: u32,     // 4  (offset 32..35)
    pub triple_indirect: u32,     // 4  (offset 36..39)
    pub is_directory: u8,         // 1  (offset 40)
    pub _reserved: [u8; 7],       // 7  (offset 41..47)
}

// #[repr(C)]
// pub struct Directory
