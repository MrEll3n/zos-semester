use crate::fs::consts::{BLOCK_SIZE, FS_MAGIC, INODE_SIZE};
use crate::fs::layout::Superblock;
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
    let mut block0 = vec![0u8; BLOCK_SIZE as usize];
    block0[0..7].copy_from_slice(&sb.fs_size.to_le_bytes());
    block0[8..11].copy_from_slice(&sb.magic);
    block0[12..15].copy_from_slice(&sb.root_inode_id.to_le_bytes());
    block0[16..19].copy_from_slice(&sb.block_start.to_le_bytes());
    block0[20..23].copy_from_slice(&sb.block_count.to_le_bytes());
    block0[24..27].copy_from_slice(&sb.inode_start.to_le_bytes());
    block0[28..31].copy_from_slice(&sb.inode_count.to_le_bytes());

    write_block(f, BLOCK_SIZE, 0, &block0)?;
    Ok(())
}

pub fn read_superblock(f: &mut File) -> std::io::Result<Superblock> {
    let mut block0 = vec![0u8; BLOCK_SIZE as usize];
    read_block(f, BLOCK_SIZE, 0, &mut block0)?;
    if &block0[0..4] != FS_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad FS magic"));
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

pub fn compute_layout(
    fs_bytes: u64,
    block_size: u32,
    bpi: u32,
) -> (
    Superblock,
    u32, /*inode_tbl_blocks*/
    u32, /*bitmap_blocks*/
) {
    let bs = block_size as u64;
    let total_blocks = (fs_bytes / bs) as u32;

    // inode count per BPI (bytes-per-inode)
    let inode_count = ((fs_bytes / (bpi as u64)).max(1)) as u32;
    let inode_table_bytes = (inode_count as u64) * (INODE_SIZE as u64);
    let inode_table_blocks = ((inode_table_bytes + bs - 1) / bs) as u32;

    let mut bitmap_blocks = 1u32;
    for _ in 0..3 {
        let overhead_wo_bitmap = 1 + inode_table_blocks;
        let data_blocks_guess = total_blocks.saturating_sub(overhead_wo_bitmap + bitmap_blocks);
        let bits = data_blocks_guess as u64;
        let bytes = (bits + 7) / 8;
        bitmap_blocks = ((bytes + bs - 1) / bs) as u32;
        if bitmap_blocks == 0 {
            bitmap_blocks = 1;
        }
    }

    let data_start = 1 + bitmap_blocks + inode_table_blocks;

    let sb = Superblock {
        magic: FS_MAGIC,
        version: FS_VERSION,
        block_size: block_size as u16,
        total_blocks,
        bitmap_start: 1,
        bitmap_blocks,
        inode_table_start: 1 + bitmap_blocks,
        inode_table_blocks,
        inode_count,
        data_start,
        root_inode_id: 0,
    };
    (sb, inode_table_blocks, bitmap_blocks)
}
