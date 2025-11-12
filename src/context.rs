use crate::fs::filesystem::FileSystem;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

pub struct Context {
    pub(crate) fs: Option<FileSystem>,
    pub(crate) fs_path: Option<PathBuf>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fs: None,
            fs_path: None,
        }
    }

    pub fn open_fs<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        // Opens (or creates) underlying image file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        // Fills instance's attributes
        self.fs_path = Some(path.as_ref().to_path_buf());

        let fs = FileSystem::open(file)?;
        self.fs = Some(fs);
        Ok(())
    }

    pub fn close_fs(&mut self) {
        if let Some(fs) = self.fs.as_mut() {
            let _ = fs.flush();
        }
        // Drop the in-memory filesystem but KEEP the last used path so commands like `format`
        // can reuse it without requiring a re-open.
        self.fs = None;
        // self.fs_path is intentionally preserved here.
    }

    pub fn fs_mut(&mut self) -> io::Result<&mut FileSystem> {
        self.fs
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Filesystem is not opened"))
    }

    pub fn fs_path(&self) -> Option<&Path> {
        self.fs_path.as_deref()
    }
}
