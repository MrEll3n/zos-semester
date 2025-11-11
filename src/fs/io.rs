use crate::fs::consts::{BLOCK_SIZE, FS_MAGIC, INODE_SIZE};
use crate::fs::layout::{Inode, Superblock};
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
    // Serialize in the exact order defined in layout.rs:
    // fs_size, magic, root_inode_id, bitmap_start, bitmap_count,
    // block_start, block_count, inode_start, inode_count
    let mut block0 = vec![0u8; BLOCK_SIZE as usize];
    block0[0..8].copy_from_slice(&sb.fs_size.to_le_bytes());
    block0[8..12].copy_from_slice(&sb.magic);
    block0[12..16].copy_from_slice(&sb.root_inode_id.to_le_bytes());
    block0[16..20].copy_from_slice(&sb.bitmap_start.to_le_bytes());
    block0[20..24].copy_from_slice(&sb.bitmap_count.to_le_bytes());
    block0[24..28].copy_from_slice(&sb.block_start.to_le_bytes());
    block0[28..32].copy_from_slice(&sb.block_count.to_le_bytes());
    block0[32..36].copy_from_slice(&sb.inode_start.to_le_bytes());
    block0[36..40].copy_from_slice(&sb.inode_count.to_le_bytes());

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
    let bitmap_start = u32::from_le_bytes(block0[16..20].try_into().unwrap());
    let bitmap_count = u32::from_le_bytes(block0[20..24].try_into().unwrap());
    let block_start = u32::from_le_bytes(block0[24..28].try_into().unwrap());
    let block_count = u32::from_le_bytes(block0[28..32].try_into().unwrap());
    let inode_start = u32::from_le_bytes(block0[32..36].try_into().unwrap());
    let inode_count = u32::from_le_bytes(block0[36..40].try_into().unwrap());

    Ok(Superblock {
        fs_size,
        magic,
        root_inode_id,
        bitmap_start,
        bitmap_count,
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
            block_start: 1,
            block_count: 0,
            inode_start: 1,
            inode_count: 0,
            bitmap_start: 1,
            bitmap_count: 0,
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

    // Estimate number of bitmap blocks required to track data blocks.
    // Each bitmap block holds (block_size_bytes * 8) bits -> that many data blocks.
    let mut bitmap_blocks: u32 = 0;
    for _ in 0..3 {
        let data_blocks_tmp = blocks_total.saturating_sub(inode_table_blocks_final + bitmap_blocks);
        let bits_per_bitmap_block = (block_size_bytes * 8) as u64;
        let needed = ((data_blocks_tmp as u64) + bits_per_bitmap_block - 1) / bits_per_bitmap_block;
        let needed_u32 = needed as u32;
        if needed_u32 == bitmap_blocks {
            break;
        }
        bitmap_blocks = needed_u32;
    }
    let data_blocks_final = blocks_total.saturating_sub(inode_table_blocks_final + bitmap_blocks);

    // Step 4: Populate superblock
    Superblock {
        fs_size: fs_bytes,
        magic: FS_MAGIC,
        root_inode_id: 0,
        block_start: 1 + bitmap_blocks + inode_table_blocks_final,
        block_count: data_blocks_final,
        inode_start: 1 + bitmap_blocks,
        inode_count: inode_count_final,
        bitmap_start: 1,
        bitmap_count: bitmap_blocks,
    }
}

// ------------------------- Bitmap API -------------------------

pub fn load_bitmap(f: &mut File, sb: &Superblock) -> io::Result<Vec<u8>> {
    if sb.bitmap_count == 0 {
        return Ok(Vec::new());
    }
    let mut buf = vec![0u8; (sb.bitmap_count as usize) * (BLOCK_SIZE as usize)];
    read_span(
        f,
        sb.bitmap_start as u64,
        sb.bitmap_count as u64,
        BLOCK_SIZE,
        &mut buf,
    )?;
    Ok(buf)
}

pub fn flush_bitmap(f: &mut File, sb: &Superblock, bitmap: &[u8]) -> io::Result<()> {
    let expected = (sb.bitmap_count as usize) * (BLOCK_SIZE as usize);
    if bitmap.len() != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "bitmap length does not match superblock bitmap_count",
        ));
    }
    write_span(
        f,
        sb.bitmap_start as u64,
        sb.bitmap_count as u64,
        BLOCK_SIZE,
        bitmap,
    )
}

#[inline]
pub fn bitmap_is_set(bitmap: &[u8], idx: u32) -> bool {
    let byte = (idx / 8) as usize;
    let bit = (idx % 8) as u8;
    (bitmap[byte] & (1u8 << bit)) != 0
}

#[inline]
pub fn bitmap_set(bitmap: &mut [u8], idx: u32) {
    let byte = (idx / 8) as usize;
    let bit = (idx % 8) as u8;
    bitmap[byte] |= 1 << bit;
}

#[inline]
pub fn bitmap_clear(bitmap: &mut [u8], idx: u32) {
    let byte = (idx / 8) as usize;
    let bit = (idx % 8) as u8;
    bitmap[byte] &= !(1 << bit);
}

pub fn find_free_data_block(bitmap: &[u8], limit: u32) -> Option<u32> {
    for (i, b) in bitmap.iter().enumerate() {
        if *b != 0xFF {
            for bit in 0..8 {
                let id = (i as u32) * 8 + bit;
                if id >= limit {
                    return None;
                }
                if b & (1 << bit) == 0 {
                    return Some(id);
                }
            }
        }
    }
    None
}

pub fn alloc_data_block(bitmap: &mut [u8], sb: &Superblock) -> Option<u32> {
    if let Some(rel) = find_free_data_block(&bitmap[..], sb.block_count) {
        bitmap_set(bitmap, rel);
        Some(sb.block_start + rel)
    } else {
        None
    }
}

pub fn free_data_block(bitmap: &mut [u8], sb: &Superblock, abs_block: u32) -> io::Result<()> {
    if abs_block < sb.block_start {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "block before data area",
        ));
    }
    let rel = abs_block - sb.block_start;
    if rel >= sb.block_count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "block beyond data area",
        ));
    }
    bitmap_clear(bitmap, rel);
    Ok(())
}

// --------------------- End of Bitmap API ----------------------

pub fn read_inode(f: &mut File, sb: &Superblock, inode_id: u32) -> io::Result<Inode> {
    if inode_id >= sb.inode_count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "inode_id out of range",
        ));
    }

    let block_size_bytes = BLOCK_SIZE as u64;
    let inode_table_base = (sb.inode_start as u64) * block_size_bytes;
    let inode_offset = inode_table_base + (inode_id as u64) * (INODE_SIZE as u64);

    let mut buf = vec![0u8; INODE_SIZE];
    f.seek(SeekFrom::Start(inode_offset))?;
    f.read_exact(&mut buf)?;

    let file_size = u64::from_le_bytes(buf[0..8].try_into().unwrap());
    let id = u32::from_le_bytes(buf[8..12].try_into().unwrap());

    let mut single_directs = [0u32; 5];
    for i in 0..5 {
        let start = 12 + i * 4;
        single_directs[i] = u32::from_le_bytes(buf[start..start + 4].try_into().unwrap());
    }

    let double_indirect = u32::from_le_bytes(buf[32..36].try_into().unwrap());
    let triple_indirect = u32::from_le_bytes(buf[36..40].try_into().unwrap());
    let is_directory = buf[40];
    let mut _reserved = [0u8; 7];
    _reserved.copy_from_slice(&buf[41..48]);

    Ok(Inode {
        file_size,
        id,
        single_directs,
        double_indirect,
        triple_indirect,
        is_directory,
        _reserved,
    })
}

pub fn write_inode(f: &mut File, sb: &Superblock, inode_id: u32, inode: &Inode) -> io::Result<()> {
    if inode_id >= sb.inode_count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "inode_id out of range",
        ));
    }

    let block_size_bytes = BLOCK_SIZE as u64;
    let inode_table_base = (sb.inode_start as u64) * block_size_bytes;
    let inode_offset = inode_table_base + (inode_id as u64) * (INODE_SIZE as u64);

    let mut buf = vec![0u8; INODE_SIZE];

    // Serialize fields to little-endian byte layout
    buf[0..8].copy_from_slice(&inode.file_size.to_le_bytes());
    buf[8..12].copy_from_slice(&inode.id.to_le_bytes());
    for i in 0..5 {
        let start = 12 + i * 4;
        buf[start..start + 4].copy_from_slice(&inode.single_directs[i].to_le_bytes());
    }
    buf[32..36].copy_from_slice(&inode.double_indirect.to_le_bytes());
    buf[36..40].copy_from_slice(&inode.triple_indirect.to_le_bytes());
    buf[40] = inode.is_directory;
    buf[41..48].copy_from_slice(&inode._reserved);

    f.seek(SeekFrom::Start(inode_offset))?;
    f.write_all(&buf)?;
    Ok(())
}

