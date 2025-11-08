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

pub fn compute_layout(fs_bytes: u64, block_size: u32, bytes_per_inode: u32) -> Superblock {
    let block_size_bytes = block_size as u64;
    let inode_size_bytes = INODE_SIZE as u64;

    // BPI (bytes) -> K (blocks per inode), min 1
    let bpi_bytes = (bytes_per_inode as u64).max(1);
    let avg_data_blocks_per_inode = ((bpi_bytes) / block_size_bytes).max(1) as u32;

    // Total block count (including superblock) and usable blocks excluding superblock
    let blocks_total_sb = (fs_bytes / block_size_bytes) as u32;
    let blocks_total = blocks_total_sb.saturating_sub(1); // exclude superblock

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

    // Step 1: Estimate inode count: I_est = floor((usable_blocks*block_size_bytes) / (avg_data_blocks_per_inode*block_size_bytes + inode_size_bytes))
    let denom = (avg_data_blocks_per_inode as u64)
        .saturating_mul(block_size_bytes)
        .saturating_add(inode_size_bytes) as u128;
    let inode_count_est =
        ((blocks_total as u128).saturating_mul(block_size_bytes as u128) / denom) as u32;
    let inode_count_est = inode_count_est.max(1);

    // Step 2: Inode table and data blocks (estimate)
    let inode_table_blocks_est =
        ((inode_count_est as u64).saturating_mul(inode_size_bytes) + block_size_bytes - 1)
            / block_size_bytes;
    let inode_table_blocks_est_u32 = (inode_table_blocks_est as u32).min(blocks_total);
    let data_blocks_est = blocks_total.saturating_sub(inode_table_blocks_est_u32);

    // Step 3: One-shot correction: ensure I_final <= floor(D_est / K)
    let inode_count_final = inode_count_est.min(if avg_data_blocks_per_inode > 0 {
        data_blocks_est / avg_data_blocks_per_inode
    } else {
        data_blocks_est
    });
    let inode_table_blocks_final =
        (((inode_count_final as u64).saturating_mul(inode_size_bytes) + block_size_bytes - 1)
            / block_size_bytes) as u32;
    let data_blocks_final = blocks_total.saturating_sub(inode_table_blocks_final);

    // Step 4: Populate superblock
    Superblock {
        fs_size: fs_bytes,
        magic: FS_MAGIC,
        root_inode_id: 0,
        inode_start: 1,
        inode_count: inode_count_final,
        block_start: 1 + inode_table_blocks_final,
        block_count: data_blocks_final,
    }
}
