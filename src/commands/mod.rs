use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub struct Context {
    fs_file: Option<File>,
    fs_path: Option<PathBuf>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fs_file: None,
            fs_path: None,
        }
    }

    pub fn open_fs<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        // Opens a file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        // Fills instance's attributes
        self.fs_path = Some(path.as_ref().to_path_buf());
        self.fs_file = Some(file);
        Ok(())
    }

    pub fn close_fs(&mut self) {
        self.fs_file = None;
        self.fs_path = None;
    }

    pub fn fs_mut_file(&mut self) -> io::Result<&mut File> {
        self.fs_file
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Filesystem file is not opened"))
    }

    pub fn fs_path(&self) -> Option<&Path> {
        self.fs_path.as_deref()
    }
}

type Handler = fn(&[&str], &mut Context);

pub struct Registry {
    pub map: HashMap<&'static str, Handler>,
}

impl Registry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert(
            "command1",
            crate::commands::command1::handle_argv as Handler,
        );
        map.insert("exit", crate::commands::exit::handle_argv as Handler);

        Self { map }
    }

    pub fn dispatch(&self, name: &str, argv: &[&str], context: &mut Context) {
        if let Some(handler) = self.map.get(name) {
            handler(argv, context);
        } else {
            eprintln!("Unknown command: {name}");
        }
    }
}

pub mod command1;
pub mod exit;
