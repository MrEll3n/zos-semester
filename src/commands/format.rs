//! `format` command implementation.
//!
//! Usage:
//!   format <SIZE>
//!
//! SIZE examples (decimal, not power-of-two unless you choose):
//!   600MB
//!   100MB
//!   1GB
//!   4096KB
//!   1048576B
//!
//! Output (per assignment spec):
//!   OK
//!   CANNOT CREATE FILE
//!
//! Semantics:
//! - Formats (reinitializes) the currently opened filesystem image to the
//!   requested size (truncates/extends underlying file).
//! - Recomputes layout (superblock + bitmap + inode table + data area).
//! - Zeros bitmap blocks and inode table blocks.
//! - Initializes root inode (id 0) as an empty directory.
//! - Replaces the `FileSystem` instance in `Context` with a freshly opened one.
//!
//! NOTE:
//! - Requires that a filesystem path was already opened via program arguments
//!   (context.fs_path must be Some). If not present, prints CANNOT CREATE FILE.
//! - Size parser supports suffixes: B, KB, MB, GB (case-insensitive).
//! - Uses DEFAULT_BPI for layout heuristic.
//!
//! Assumptions / Simplifications:
//! - No inode bitmap (free inode recognized by link_count == 0).
//! - Direct blocks only (5) for now.
//!
//! Future improvements:
//! - Validate minimal size (e.g. at least one block beyond superblock).
//! - Add explicit error variants if needed by assignment.
//!
use crate::context::Context;
use crate::fs::consts::{BLOCK_SIZE, DEFAULT_BPI, INODE_SIZE};
use crate::fs::filesystem::FileSystem;
use crate::fs::io::{compute_layout, write_inode, write_span, write_superblock};
use crate::fs::layout::Inode;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // DEBUG: starting format command
    // If a filesystem is already open, close (flush + drop) it before reformatting the image.
    if context.fs.is_some() {
        eprintln!("DBG format: closing existing filesystem before reinitialization");
        context.close_fs();
    }
    // Expect exactly one argument: size
    if argv.len() != 1 {
        eprintln!("CANNOT CREATE FILE");
        return;
    }
    let size_str = argv[0];

    // Parse size string to bytes
    let fs_bytes = match parse_size(size_str) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
    };

    // Need an existing path (opened or at least known)
    let path = match context.fs_path() {
        Some(p) => p.to_path_buf(),
        None => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
    };

    // (Re)open file with read/write, create if missing
    let mut file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(_) => {
            eprintln!("CANNOT CREATE FILE");
            return;
        }
    };

    // Resize underlying file to requested size
    if let Err(_) = file.set_len(fs_bytes) {
        eprintln!("CANNOT CREATE FILE");
        return;
    }

    // Compute layout
    let sb = compute_layout(fs_bytes, BLOCK_SIZE, DEFAULT_BPI);
    eprintln!(
        "DBG format: layout fs_size={} block_count={} bitmap_count={} inode_count={} inode_start={} block_start={}",
        sb.fs_size, sb.block_count, sb.bitmap_count, sb.inode_count, sb.inode_start, sb.block_start
    );

    // Write superblock (block 0)
    if let Err(e) = write_superblock(&mut file, &sb) {
        eprintln!("DBG format: write_superblock failed: {:?}", e);
        eprintln!("CANNOT CREATE FILE");
        return;
    }

    // Zero bitmap blocks (if any)
    if sb.bitmap_count > 0 {
        let bitmap_bytes = (sb.bitmap_count as usize) * (BLOCK_SIZE as usize);
        eprintln!(
            "DBG format: zeroing bitmap blocks count={} total_bytes={}",
            sb.bitmap_count, bitmap_bytes
        );
        let zero_bitmap = vec![0u8; bitmap_bytes];
        if let Err(e) = write_span(
            &mut file,
            sb.bitmap_start as u64,
            sb.bitmap_count as u64,
            BLOCK_SIZE,
            &zero_bitmap,
        ) {
            eprintln!("DBG format: bitmap zeroing failed: {:?}", e);
            eprintln!("CANNOT CREATE FILE");
            return;
        }
        eprintln!("DBG format: bitmap zeroed successfully");
    } else {
        eprintln!("DBG format: no bitmap blocks (bitmap_count=0)");
    }

    // Zero inode table blocks
    // Inode table block count = sb.block_start - sb.inode_start
    let inode_table_block_count = sb.block_start.saturating_sub(sb.inode_start);
    if inode_table_block_count > 0 {
        let inode_bytes = (inode_table_block_count as usize) * (BLOCK_SIZE as usize);
        eprintln!(
            "DBG format: zeroing inode table blocks count={} total_bytes={}",
            inode_table_block_count, inode_bytes
        );
        let zero_inode_blocks =
            vec![0u8; (inode_table_block_count as usize) * (BLOCK_SIZE as usize)];
        if let Err(e) = write_span(
            &mut file,
            sb.inode_start as u64,
            inode_table_block_count as u64,
            BLOCK_SIZE,
            &zero_inode_blocks,
        ) {
            eprintln!("DBG format: inode table zeroing failed: {:?}", e);
            eprintln!("CANNOT CREATE FILE");
            return;
        }
        eprintln!("DBG format: inode table zeroed successfully");
    } else {
        eprintln!("DBG format: no inode table blocks to zero (inode_table_block_count=0)");
    }

    // Initialize root inode (id = sb.root_inode_id, usually 0)
    if sb.inode_count == 0 {
        // No inode space -> invalid FS layout
        eprintln!("CANNOT CREATE FILE");
        return;
    }

    let root_id = sb.root_inode_id;

    let root_inode = Inode {
        file_size: 0,

        id: root_id,

        single_directs: [0u32; 5],

        single_indirect: 0,

        double_indirect: 0,

        file_type: 1, // directory

        link_count: 1,

        _reserved: [0u8; 6],
    };

    eprintln!(
        "DBG format: initializing root inode id={} type=DIR",
        root_id
    );
    if let Err(e) = write_inode(&mut file, &sb, root_id, &root_inode) {
        eprintln!("DBG format: root inode write failed: {:?}", e);
        eprintln!("CANNOT CREATE FILE");
        return;
    }
    eprintln!("DBG format: root inode initialized");

    // Ensure data past written areas is physically present (optional pre-zero)
    // Since set_len already extended/truncated, and we zeroed metadata areas,
    // data blocks will remain whatever OS provided (often zeros).

    // Reopen FileSystem (replace context.fs)
    match FileSystem::open(file) {
        Ok(fs) => {
            context.fs = Some(fs);
            eprintln!("OK");
        }
        Err(_) => {
            eprintln!("CANNOT CREATE FILE");
        }
    }
}

/// Parse a size string like "600MB", "1GB", "4096KB", "123B".
fn parse_size(s: &str) -> Result<u64, ()> {
    if s.is_empty() {
        return Err(());
    }
    // Split into numeric prefix + unit suffix
    let (num_part, unit_part) = split_number_unit(s);
    if num_part.is_empty() {
        return Err(());
    }
    let base: u64 = num_part.parse().map_err(|_| ())?;
    let unit = unit_part.to_ascii_uppercase();

    let mul = match unit.as_str() {
        "" | "B" => 1,
        "KB" => 1_024,
        "MB" => 1_024 * 1_024,
        "GB" => 1_024 * 1_024 * 1_024,
        _ => return Err(()),
    };
    Ok(base.saturating_mul(mul))
}

/// Split string into (numeric_part, unit_part) at first non-digit.
fn split_number_unit(s: &str) -> (&str, &str) {
    let mut idx = 0;
    for (i, ch) in s.char_indices() {
        if !ch.is_ascii_digit() {
            idx = i;
            break;
        }
    }
    if idx == 0 {
        // Might be all digits or starts with non-digit
        if s.chars().all(|c| c.is_ascii_digit()) {
            (s, "")
        } else {
            ("", s)
        }
    } else if idx >= s.len() {
        (s, "")
    } else {
        (&s[..idx], &s[idx..])
    }
}
