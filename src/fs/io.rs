use crate::fs::consts::{DEFAULT_BLOCK_SIZE, FS_MAGIC, FS_VERSION};
use crate::fs::superblock::Superblock;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

pub fn write_block(
    f: &mut File,
    block_size: u32,
    block_index: u64,
    buf: &[u8],
) -> std::io::Result<()> {
    debug_assert_eq!(buf.len(), block_size as usize);
    f.seek(SeekFrom::Start(block_index * block_size as u64))?;
    f.write_all(buf)?;
    Ok(())
}

pub fn read_block(
    f: &mut File,
    block_size: u32,
    block_index: u64,
    buf: &mut [u8],
) -> io::Result<()> {
    debug_assert_eq!(buf.len(), block_size as usize);
    f.seek(SeekFrom::Start(block_index * block_size as u64))?;
    f.read_exact(buf)?;
    Ok(())
}

pub fn write_span(
    f: &mut File,
    start_block: u64,
    count: u64,
    block_size: u32,
    buf: &[u8],
) -> std::io::Result<()> {
    debug_assert_eq!(buf.len(), (count as usize) * (block_size as usize));
    f.seek(SeekFrom::Start(start_block * block_size as u64))?;
    f.write_all(buf)?;
    Ok(())
}

pub fn read_span(
    f: &mut File,
    start_block: u64,
    count: u64,
    block_size: u32,
    buf: &mut [u8],
) -> std::io::Result<()> {
    debug_assert_eq!(buf.len(), (count as usize) * (block_size as usize));
    f.seek(SeekFrom::Start(start_block * block_size as u64))?;
    f.read_exact(buf)?;
    Ok(())
}

pub fn write_superblock(f: &mut File, sb: &Superblock) -> std::io::Result<()> {
    let mut block0 = vec![0u8; DEFAULT_BLOCK_SIZE as usize];
    block0[0..4].copy_from_slice(&sb.magic);
    block0[4..6].copy_from_slice(&sb.version.to_le_bytes());
    block0[6..8].copy_from_slice(&sb.block_size.to_le_bytes());
    block0[8..12].copy_from_slice(&sb.total_blocks.to_le_bytes());
    block0[12..16].copy_from_slice(&sb.bitmap_start.to_le_bytes());
    block0[16..20].copy_from_slice(&sb.bitmap_blocks.to_le_bytes());
    block0[20..24].copy_from_slice(&sb.inode_table_start.to_le_bytes());
    block0[24..28].copy_from_slice(&sb.inode_table_blocks.to_le_bytes());
    block0[28..32].copy_from_slice(&sb.inode_count.to_le_bytes());
    block0[32..36].copy_from_slice(&sb.data_start.to_le_bytes());
    block0[36..40].copy_from_slice(&sb.root_inode_id.to_le_bytes());
    write_block(f, DEFAULT_BLOCK_SIZE, 0, &block0)?;
    Ok(())
}

pub fn read_superblock(f: &mut File) -> std::io::Result<Superblock> {
    let mut block0 = vec![0u8; DEFAULT_BLOCK_SIZE as usize];
    read_block(f, DEFAULT_BLOCK_SIZE, 0, &mut block0)?;
    if &block0[0..4] != FS_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad FS magic"));
    }

    let version = u16::from_le_bytes([block0[5], block0[6]]);
    if version != FS_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad Version"));
    }

    let load_u16 = |i: usize| u16::from_le_bytes(block0[i..i + 2].try_into().unwrap());
    let load_u32 = |i: usize| u32::from_le_bytes(block0[i..i + 4].try_into().unwrap());
    Ok(Superblock {
        magic: FS_MAGIC,
        version: load_u16(4),
        block_size: load_u16(6),
        total_blocks: load_u32(8),
        bitmap_start: load_u32(12),
        bitmap_blocks: load_u32(16),
        inode_table_start: load_u32(20),
        inode_table_blocks: load_u32(24),
        inode_count: load_u32(28),
        data_start: load_u32(32),
        root_inode_id: load_u32(36),
    })
}
