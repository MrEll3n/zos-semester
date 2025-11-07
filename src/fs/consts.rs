pub const FS_MAGIC: [u8; 4] = *b"ELFS";
pub const FS_VERSION: u16 = 1;
pub const DEFAULT_BLOCK_SIZE: u32 = 4096;
pub const INODE_SIZE: usize = 64; // 64 B
pub const DEFAULT_INODE_COUNT: u32 = 8192;
pub const DEFAULT_FS_BYTES: u64 = 600 * 1024 * 1024; // 600 MiB
pub const DEFAULT_BPI: u32 = 16 * 1024; // 16 KiB
