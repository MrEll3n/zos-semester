pub struct FileSystem {
    file: std::fs::File,

    sb: crate::fs::layout::Superblock,

    data_bitmap: Vec<u8>,
    cwd_inode: u32,
    cwd_stack: Vec<u32>,
    cwd_path: String,
    bitmap_dirty: bool,
}

impl FileSystem {
    pub fn open(mut file: std::fs::File) -> std::io::Result<Self> {
        use crate::fs::io::{load_bitmap, read_superblock};
        let sb = read_superblock(&mut file)?;
        let data_bitmap = load_bitmap(&mut file, &sb)?;
        let cwd_inode = sb.root_inode_id;

        Ok(Self {
            file,

            sb,

            data_bitmap,

            cwd_inode,

            cwd_stack: Vec::new(),

            cwd_path: "/".to_string(),
            bitmap_dirty: false,
        })
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.bitmap_dirty {
            crate::fs::io::flush_bitmap(&mut self.file, &self.sb, &self.data_bitmap)?;
            self.bitmap_dirty = false;
        }
        Ok(())
    }

    // Inode helpers
    pub fn read_inode(&mut self, id: u32) -> std::io::Result<crate::fs::layout::Inode> {
        crate::fs::io::read_inode(&mut self.file, &self.sb, id)
    }
    pub fn write_inode(
        &mut self,
        id: u32,
        inode: &crate::fs::layout::Inode,
    ) -> std::io::Result<()> {
        crate::fs::io::write_inode(&mut self.file, &self.sb, id, inode)
    }

    // Block alloc/free (via bitmapu)
    pub fn alloc_block(&mut self) -> Option<u32> {
        let b = crate::fs::io::alloc_data_block(&mut self.data_bitmap, &self.sb)?;
        self.bitmap_dirty = true;
        Some(b)
    }
    pub fn free_block(&mut self, abs_block: u32) -> std::io::Result<()> {
        crate::fs::io::free_data_block(&mut self.data_bitmap, &self.sb, abs_block)?;
        self.bitmap_dirty = true;
        Ok(())
    }

    // Inode allocation (scan link_count == 0)
    pub fn alloc_inode(&mut self) -> std::io::Result<Option<u32>> {
        for id in 1..self.sb.inode_count {
            let ino = crate::fs::io::read_inode(&mut self.file, &self.sb, id)?;
            if ino.link_count == 0 {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    pub fn free_inode(&mut self, inode_id: u32) -> std::io::Result<()> {
        let mut ino = crate::fs::io::read_inode(&mut self.file, &self.sb, inode_id)?;
        // Release direct blocks
        for b in ino.single_directs.iter_mut() {
            if *b != 0 {
                self.free_block(*b)?;
                *b = 0;
            }
        }
        // (Indirect blocks ignored in this minimal implementation)
        ino.file_size = 0;
        ino.file_type = 0;
        ino.link_count = 0;
        crate::fs::io::write_inode(&mut self.file, &self.sb, inode_id, &ino)
    }

    // Block mapping (direct pointers only)
    fn get_block(&self, inode: &crate::fs::layout::Inode, logical: u64) -> Option<u32> {
        if logical < 5 {
            let b = inode.single_directs[logical as usize];
            if b == 0 { None } else { Some(b) }
        } else {
            None
        }
    }

    fn get_or_alloc_block(
        &mut self,
        inode: &mut crate::fs::layout::Inode,
        logical: u64,
    ) -> std::io::Result<Option<u32>> {
        if logical >= 5 {
            return Ok(None); // not supported beyond 5 direct blocks
        }
        let idx = logical as usize;
        if inode.single_directs[idx] == 0 {
            if let Some(b) = self.alloc_block() {
                inode.single_directs[idx] = b;
                self.write_inode(inode.id, inode)?;
                Ok(Some(b))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(inode.single_directs[idx]))
        }
    }

    // File read (range) - assumes range is within file_size
    pub fn read_file_range(
        &mut self,
        inode: &crate::fs::layout::Inode,
        offset: u64,
        buf: &mut [u8],
    ) -> std::io::Result<()> {
        use std::cmp::min;
        let block_size = crate::fs::consts::BLOCK_SIZE as u64;
        let end = offset + buf.len() as u64;
        if end > inode.file_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read beyond file_size",
            ));
        }
        let mut remaining = buf.len();
        let mut cursor = offset;
        let mut dst_pos = 0;
        while remaining > 0 {
            let logical = cursor / block_size;
            let within = (cursor % block_size) as usize;
            let to_take = min(remaining, (block_size as usize) - within);
            let abs_block = self.get_block(inode, logical).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "missing block")
            })?;
            let mut block_buf = vec![0u8; crate::fs::consts::BLOCK_SIZE as usize];
            crate::fs::io::read_block(
                &mut self.file,
                crate::fs::consts::BLOCK_SIZE,
                abs_block as u64,
                &mut block_buf,
            )?;
            buf[dst_pos..dst_pos + to_take].copy_from_slice(&block_buf[within..within + to_take]);
            cursor += to_take as u64;
            dst_pos += to_take;
            remaining -= to_take;
        }
        Ok(())
    }

    // File write (range) - allocates direct blocks as needed
    pub fn write_file_range(
        &mut self,
        inode: &mut crate::fs::layout::Inode,
        offset: u64,
        data: &[u8],
    ) -> std::io::Result<()> {
        use std::cmp::min;
        let block_size = crate::fs::consts::BLOCK_SIZE as u64;
        let mut remaining = data.len();
        let mut cursor = offset;
        let mut src_pos = 0;
        while remaining > 0 {
            let logical = cursor / block_size;
            if logical >= 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "file too large for direct-only implementation",
                ));
            }
            let within = (cursor % block_size) as usize;
            let to_write = min(remaining, (block_size as usize) - within);

            let (abs_block, existed) = match self.get_block(inode, logical) {
                Some(b) => (b, true),

                None => {
                    let allocated = self.get_or_alloc_block(inode, logical)?.ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::Other, "no space for block")
                    })?;

                    (allocated, false)
                }
            };

            // Read-modify-write block

            let mut block_buf = vec![0u8; crate::fs::consts::BLOCK_SIZE as usize];

            if existed {
                crate::fs::io::read_block(
                    &mut self.file,
                    crate::fs::consts::BLOCK_SIZE,
                    abs_block as u64,
                    &mut block_buf,
                )?;
            }

            block_buf[within..within + to_write]
                .copy_from_slice(&data[src_pos..src_pos + to_write]);

            crate::fs::io::write_block(
                &mut self.file,
                crate::fs::consts::BLOCK_SIZE,
                abs_block as u64,
                &block_buf,
            )?;

            cursor += to_write as u64;
            src_pos += to_write;
            remaining -= to_write;
        }
        let new_end = offset + data.len() as u64;
        if new_end > inode.file_size {
            inode.file_size = new_end;
            self.write_inode(inode.id, inode)?;
        }
        Ok(())
    }

    // Directory operations (direct file content of entries)

    fn dir_entry_count(&self, dir_inode: &crate::fs::layout::Inode) -> usize {
        (dir_inode.file_size as usize) / crate::fs::consts::DIR_ENTRY_SIZE
    }

    pub(crate) fn dir_find(
        &mut self,
        dir_inode: &crate::fs::layout::Inode,
        name: &str,
    ) -> std::io::Result<Option<(usize, crate::fs::layout::DirectoryEntry)>> {
        if dir_inode.file_type != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "not a directory",
            ));
        }
        let slots = self.dir_entry_count(dir_inode);
        let mut slot_buf = vec![0u8; crate::fs::consts::DIR_ENTRY_SIZE];
        for i in 0..slots {
            self.read_file_range(
                dir_inode,
                (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                &mut slot_buf,
            )?;
            let entry = crate::fs::layout::DirectoryEntry::deserialize(&slot_buf);
            if !entry.is_unused() {
                let end = entry
                    .name
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(crate::fs::consts::DIR_NAME_LEN);
                let entry_name = std::str::from_utf8(&entry.name[..end]).unwrap_or("");
                if entry_name == name {
                    return Ok(Some((i, entry)));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn dir_is_empty(
        &mut self,
        dir_inode: &crate::fs::layout::Inode,
    ) -> std::io::Result<bool> {
        let slots = self.dir_entry_count(dir_inode);
        let mut slot_buf = vec![0u8; crate::fs::consts::DIR_ENTRY_SIZE];
        for i in 0..slots {
            self.read_file_range(
                dir_inode,
                (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                &mut slot_buf,
            )?;
            let entry = crate::fs::layout::DirectoryEntry::deserialize(&slot_buf);
            if !entry.is_unused() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn dir_add_entry(
        &mut self,
        dir_inode: &mut crate::fs::layout::Inode,
        name: &str,
        inode_id: u32,
    ) -> std::io::Result<()> {
        if name.is_empty() || name.len() > crate::fs::consts::DIR_NAME_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid name length",
            ));
        }
        if let Some(_) = self.dir_find(dir_inode, name)? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "entry exists",
            ));
        }
        let slots = self.dir_entry_count(dir_inode);
        let mut slot_buf = vec![0u8; crate::fs::consts::DIR_ENTRY_SIZE];
        // Try free slot
        for i in 0..slots {
            self.read_file_range(
                dir_inode,
                (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                &mut slot_buf,
            )?;
            let entry = crate::fs::layout::DirectoryEntry::deserialize(&slot_buf);
            if entry.is_unused() {
                let new_e = crate::fs::layout::DirectoryEntry::from_name(name, inode_id).unwrap();
                new_e.serialize(&mut slot_buf);
                self.write_file_range(
                    dir_inode,
                    (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                    &slot_buf,
                )?;
                self.write_inode(dir_inode.id, dir_inode)?;
                return Ok(());
            }
        }
        // Append new slot
        let new_e = crate::fs::layout::DirectoryEntry::from_name(name, inode_id).unwrap();
        new_e.serialize(&mut slot_buf);

        let offset = dir_inode.file_size;

        self.write_file_range(dir_inode, offset, &slot_buf)?;

        // write_file_range updates file_size; avoid double increment
        self.write_inode(dir_inode.id, dir_inode)?;

        Ok(())
    }

    pub fn dir_remove_entry(
        &mut self,
        dir_inode: &mut crate::fs::layout::Inode,
        name: &str,
    ) -> std::io::Result<()> {
        let slots = self.dir_entry_count(dir_inode);
        let mut slot_buf = vec![0u8; crate::fs::consts::DIR_ENTRY_SIZE];
        for i in 0..slots {
            self.read_file_range(
                dir_inode,
                (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                &mut slot_buf,
            )?;
            let mut entry = crate::fs::layout::DirectoryEntry::deserialize(&slot_buf);
            let end = entry
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(crate::fs::consts::DIR_NAME_LEN);
            let entry_name = std::str::from_utf8(&entry.name[..end]).unwrap_or("");
            if !entry.is_unused() && entry_name == name {
                entry.mark_unused();
                entry.serialize(&mut slot_buf);
                self.write_file_range(
                    dir_inode,
                    (i * crate::fs::consts::DIR_ENTRY_SIZE) as u64,
                    &slot_buf,
                )?;
                self.write_inode(dir_inode.id, dir_inode)?;
                return Ok(());
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "entry not found",
        ))
    }

    // Symlink target reader (returns UTF-8 path stored in the symlink inode)

    pub fn readlink_target(&mut self, inode_id: u32) -> std::io::Result<String> {
        let inode = self.read_inode(inode_id)?;

        if inode.file_type != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a symlink",
            ));
        }

        let size = inode.file_size as usize;

        let mut buf = vec![0u8; size];

        self.read_file_range(&inode, 0, &mut buf)?;

        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    // Public path resolver using parent stack; returns final inode id.
    pub fn resolve_path(&mut self, path: &str) -> std::io::Result<u32> {
        let (id, _) = self.resolve_path_with_stack(path)?;
        Ok(id)
    }

    // Resolve path and return also resulting parent stack.
    fn resolve_path_with_stack(&mut self, path: &str) -> std::io::Result<(u32, Vec<u32>)> {
        let max_depth = 16;

        if path.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "empty path",
            ));
        }

        let mut comps: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
        let is_abs = path.starts_with('/');
        let mut current_id = if is_abs {
            self.sb.root_inode_id
        } else {
            self.cwd_inode
        };

        let mut parents: Vec<u32> = if is_abs {
            Vec::new()
        } else {
            self.cwd_stack.clone()
        };

        self.resolve_components_with_stack(current_id, &mut parents, &mut comps, 0, max_depth)
            .map(|final_id| (final_id, parents))
    }

    // Resolve parent and final name component (does not require that final exists).
    pub fn resolve_parent_and_name(&mut self, path: &str) -> std::io::Result<(u32, String)> {
        if path.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "empty path",
            ));
        }
        let mut comps: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();

        if comps.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "missing name",
            ));
        }
        let name = comps.pop().unwrap().to_string();
        let is_abs = path.starts_with('/');
        let mut parents: Vec<u32> = if is_abs {
            Vec::new()
        } else {
            self.cwd_stack.clone()
        };
        let start = if is_abs {
            self.sb.root_inode_id
        } else {
            self.cwd_inode
        };
        let max_depth = 16;

        let parent_id =
            self.resolve_components_with_stack(start, &mut parents, &mut comps, 0, max_depth)?;

        Ok((parent_id, name))
    }

    pub(crate) fn cd(&mut self, path: &str) -> std::io::Result<()> {
        let (id, stack) = self.resolve_path_with_stack(path)?;

        let inode = self.read_inode(id)?;

        if inode.file_type != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not a directory",
            ));
        }

        // Reconstruct path string from stack + current
        if id == self.sb.root_inode_id {
            self.cwd_path = "/".to_string();
        } else {
            // We only have inode IDs in stack; simplest approach: if we cannot map IDs back to names,
            // fall back to the provided input path normalization:
            // If absolute -> normalize; if relative -> append to existing and re-normalize.
            let mut new_path = if path.starts_with('/') {
                path.to_string()
            } else {
                if self.cwd_path.ends_with('/') {
                    format!("{}{}", self.cwd_path, path)
                } else {
                    format!("{}/{}", self.cwd_path, path)
                }
            };
            // Normalize: collapse //, /./, and trailing / (except root)
            let mut parts = Vec::new();
            for comp in new_path.split('/') {
                if comp.is_empty() || comp == "." {
                    continue;
                }
                if comp == ".." {
                    if !parts.is_empty() {
                        parts.pop();
                    }
                    continue;
                }
                parts.push(comp);
            }
            if parts.is_empty() {
                new_path = "/".to_string();
            } else {
                new_path = format!("/{}", parts.join("/"));
            }
            self.cwd_path = new_path;
        }
        self.cwd_inode = id;
        self.cwd_stack = stack;
        Ok(())
    }

    // Core resolver that traverses components with a parent stack and symlink expansion.
    fn resolve_components_with_stack(
        &mut self,
        mut current_id: u32,
        parents: &mut Vec<u32>,
        comps: &mut Vec<&str>,
        depth: usize,
        max_depth: usize,
    ) -> std::io::Result<u32> {
        if depth > max_depth {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "symlink depth exceeded",
            ));
        }

        let mut idx = 0;
        while idx < comps.len() {
            let comp = comps[idx];

            if comp == "." {
                idx += 1;
                continue;
            }
            if comp == ".." {
                if let Some(parent) = parents.pop() {
                    current_id = parent;
                } else {
                    current_id = self.sb.root_inode_id;
                }
                idx += 1;
                continue;
            }

            let cur_inode = self.read_inode(current_id)?;
            if cur_inode.file_type != 1 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not a directory",
                ));
            }

            match self.dir_find(&cur_inode, comp)? {
                Some((_, entry)) => {
                    let next_id = entry.inode_id;

                    let next_inode = self.read_inode(next_id)?;

                    if next_inode.file_type == 2 {
                        let target = self.readlink_target(next_id)?;

                        let remaining: Vec<&str> = comps[idx + 1..].iter().copied().collect();
                        let mut new_comps: Vec<&str> =
                            target.split('/').filter(|c| !c.is_empty()).collect();

                        new_comps.extend(remaining);
                        if target.starts_with('/') {
                            parents.clear();

                            return self.resolve_components_with_stack(
                                self.sb.root_inode_id,
                                parents,
                                &mut new_comps,
                                depth + 1,
                                max_depth,
                            );
                        } else {
                            return self.resolve_components_with_stack(
                                current_id,
                                parents,
                                &mut new_comps,
                                depth + 1,
                                max_depth,
                            );
                        }
                    } else {
                        parents.push(current_id);
                        current_id = next_id;
                        idx += 1;
                    }
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "component not found",
                    ));
                }
            }
        }

        Ok(current_id)
    }

    pub(crate) fn pwd(&self) -> &str {
        &self.cwd_path
    }
}
