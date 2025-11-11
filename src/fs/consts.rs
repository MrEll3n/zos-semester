pub const FS_MAGIC: [u8; 4] = *b"ELFS";
pub const INODE_SIZE: usize = 48; // 48 B
pub const BLOCK_SIZE: u32 = 4 * 1024; // 4 KiB
pub const DEFAULT_FS_BYTES: u64 = 600 * 1024 * 1024; // 600 MiB
// based of ext default BPI - Bytes per Inode
// lower BPI -> more inodes = good for small files
// higher BPI -> less inodes = good for bigger files
pub const DEFAULT_BPI: u32 = 16 * 1024; // 16 KiB
pub const DIR_NAME_LEN: usize = 12;
pub const DIR_ENTRY_SIZE: usize = 16;
pub const DIR_INODE_UNUSED: u32 = u32::MAX;
