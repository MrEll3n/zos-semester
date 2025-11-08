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
    block0[0..8].copy_from_slice(&sb.fs_size.to_le_bytes());
    block0[8..12].copy_from_slice(&sb.magic);
    block0[12..16].copy_from_slice(&sb.root_inode_id.to_le_bytes());
    block0[16..20].copy_from_slice(&sb.block_start.to_le_bytes());
    block0[20..24].copy_from_slice(&sb.block_count.to_le_bytes());
    block0[24..28].copy_from_slice(&sb.inode_start.to_le_bytes());
    block0[28..32].copy_from_slice(&sb.inode_count.to_le_bytes());

    write_block(f, BLOCK_SIZE, 0, &block0)?;
    Ok(())
}

pub fn read_superblock(f: &mut File) -> std::io::Result<Superblock> {
    let mut block0 = vec![0u8; BLOCK_SIZE as usize];
    read_block(f, BLOCK_SIZE, 0, &mut block0)?;

    let fs_size = u64::from_le_bytes(block0[0..8].try_into().unwrap());
    let magic: [u8; 4] = block0[8..12].try_into().unwrap();
    if magic != FS_MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad FS magic"));
    }

    let root_inode_id = u32::from_le_bytes(block0[12..16].try_into().unwrap());
    let block_start = u32::from_le_bytes(block0[16..20].try_into().unwrap());
    let block_count = u32::from_le_bytes(block0[20..24].try_into().unwrap());
    let inode_start = u32::from_le_bytes(block0[24..28].try_into().unwrap());
    let inode_count = u32::from_le_bytes(block0[28..32].try_into().unwrap());

    Ok(Superblock {
        fs_size,
        magic,
        root_inode_id,
        block_start,
        block_count,
        inode_start,
        inode_count,
    })
}

pub fn compute_layout(fs_bytes: u64, block_size: u32, avg_blocks_per_inode: u32) -> Superblock {
    let block_size = block_size as u64;
    let inode_size = INODE_SIZE as u64;
    let bpi = avg_blocks_per_inode.max(1) as u64;

    // 1.
    let blocks_total_sb = (fs_bytes / block_size) as u32;
    let blocks_total = blocks_total_sb.saturating_sub(1); // without SB

    if blocks_total == 0 {
        return Superblock {
            fs_size: fs_bytes,
            magic: FS_MAGIC,
            root_inode_id: 0,
            inode_start: 1,
            inode_count: 0,
            block_start: 1,
            block_count: 0,
        };
    }

    // 1) Inode count estimation
    let denom = bpi.saturating_mul(block_size).saturating_add(inode_size) as u128;
    let inode_count_est =
        ((blocks_total as u128).saturating_mul(block_size as u128) / denom) as u32;
    let inode_count_est = inode_count_est.max(1);

    // 2) Inode size and Data size estimation
    let inode_size_est =
        ((inode_count_est as u64).saturating_mul(inode_size) + block_size - 1) / block_size;
    let inode_size_est_u32 = (inode_size_est as u32).min(blocks_total);
    let data_size_est = blocks_total.saturating_sub(inode_size_est_u32);

    // 3) one time correction
    let inode_count_final = inode_count_est.min(if bpi > 0 {
        data_size_est / (bpi as u32)
    } else {
        data_size_est
    });
    let inode_size_final = (((inode_count_final as u64).saturating_mul(inode_size) + block_size
        - 1)
        / block_size) as u32;
    let data_size_final = blocks_total.saturating_sub(inode_size_final);

    // 4) Write into the superblock
    Superblock {
        fs_size: fs_bytes,
        magic: FS_MAGIC,
        root_inode_id: 0,
        inode_start: 1,
        inode_count: inode_count_final,
        block_start: 1 + inode_size_final,
        block_count: data_size_final,
    }
}
