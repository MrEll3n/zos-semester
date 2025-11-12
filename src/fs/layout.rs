use crate::fs::consts::{DIR_ENTRY_SIZE, DIR_INODE_UNUSED, DIR_NAME_LEN, INODE_SIZE};
use std::fmt;

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
    pub file_size: u64,           // 8  (offset 0..7)
    pub id: u32,                  // 4  (offset 8..11)
    pub single_directs: [u32; 5], // 20 (offset 12..31)
    pub single_indirect: u32,     // 4  (offset 32..35) 1st level of indirection
    pub double_indirect: u32,     // 4  (offset 36..39) 2nd level of indirection
    pub file_type: u8,            // 1  (offset 40) | 0 - file, 1 - dir, 2 - symlink
    pub link_count: u8,           // 1  (offset 41)
    pub _reserved: [u8; 6],       // 6  (offset 42..47) remaining padding
}

impl Inode {
    pub fn to_bytes(&self) -> [u8; INODE_SIZE] {
        let mut buf = [0u8; INODE_SIZE];
        // Fixed fields
        buf[0..8].copy_from_slice(&self.file_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.id.to_le_bytes());
        // Direct blocks
        for i in 0..5 {
            let start = 12 + i * 4;
            buf[start..start + 4].copy_from_slice(&self.single_directs[i].to_le_bytes());
        }
        // Indirect levels
        buf[32..36].copy_from_slice(&self.single_indirect.to_le_bytes());
        buf[36..40].copy_from_slice(&self.double_indirect.to_le_bytes());
        // Metadata
        buf[40] = self.file_type;
        buf[41] = self.link_count;
        buf[42..48].copy_from_slice(&self._reserved);
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Self {
        debug_assert_eq!(buf.len(), INODE_SIZE);
        let file_size = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let id = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        let mut single_directs = [0u32; 5];
        for i in 0..5 {
            let start = 12 + i * 4;
            single_directs[i] = u32::from_le_bytes(buf[start..start + 4].try_into().unwrap());
        }
        let single_indirect = u32::from_le_bytes(buf[32..36].try_into().unwrap());
        let double_indirect = u32::from_le_bytes(buf[36..40].try_into().unwrap());
        let file_type = buf[40];
        let link_count = buf[41];
        let mut _reserved = [0u8; 6];
        _reserved.copy_from_slice(&buf[42..48]);
        Self {
            file_size,
            id,
            single_directs,
            single_indirect,
            double_indirect,
            file_type,
            link_count,
            _reserved,
        }
    }
}

#[repr(C)]
pub struct DirectoryEntry {
    pub name: [u8; DIR_NAME_LEN],
    pub inode_id: u32,
}

impl DirectoryEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; DIR_NAME_LEN],
            inode_id: DIR_INODE_UNUSED,
        }
    }

    pub fn is_unused(&self) -> bool {
        self.inode_id == DIR_INODE_UNUSED
    }

    pub fn from_name(name: &str, inode_id: u32) -> Result<Self, &'static str> {
        if name.is_empty() || name.len() > DIR_NAME_LEN {
            return Err("invalid name length");
        }

        let mut buf = [0u8; DIR_NAME_LEN];
        buf[..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self {
            name: buf,
            inode_id,
        })
    }

    pub fn name_str(&self) -> &str {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(DIR_NAME_LEN);

        std::str::from_utf8(&self.name[..end]).unwrap_or("<invalid>")
    }

    pub fn mark_unused(&mut self) {
        self.inode_id = DIR_INODE_UNUSED;
    }

    pub fn serialize(&self, out: &mut [u8]) {
        debug_assert_eq!(out.len(), DIR_ENTRY_SIZE);
        out[0..DIR_NAME_LEN].copy_from_slice(&self.name);
        out[DIR_NAME_LEN..DIR_NAME_LEN + 4].copy_from_slice(&self.inode_id.to_le_bytes());
    }

    pub fn deserialize(inp: &[u8]) -> Self {
        debug_assert_eq!(inp.len(), DIR_ENTRY_SIZE);
        let mut name = [0u8; DIR_NAME_LEN];
        name.copy_from_slice(&inp[0..DIR_NAME_LEN]);
        let inode_id = u32::from_le_bytes(inp[DIR_NAME_LEN..DIR_NAME_LEN + 4].try_into().unwrap());

        Self { name, inode_id }
    }
}

impl fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_unused() {
            write!(f, "DirectoryEntry(<free>)")
        } else {
            write!(
                f,
                "DirectoryEntry({}, inode={})",
                self.name_str(),
                self.inode_id
            )
        }
    }
}
