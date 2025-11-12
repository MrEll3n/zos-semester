use crate::context::Context;
use std::collections::HashMap;

type Handler = fn(&[&str], &mut Context);

pub struct Registry {
    pub map: HashMap<&'static str, Handler>,
}

impl Registry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert("exit", crate::commands::exit::handle_argv as Handler);
        map.insert("cd", crate::commands::cd::handle_argv as Handler);
        map.insert("pwd", crate::commands::pwd::handle_argv as Handler);
        map.insert("mkdir", crate::commands::mkdir::handle_argv as Handler);
        map.insert("rmdir", crate::commands::rmdir::handle_argv as Handler);
        map.insert("ls", crate::commands::ls::handle_argv as Handler);
        map.insert("cat", crate::commands::cat::handle_argv as Handler);
        map.insert("rm", crate::commands::rm::handle_argv as Handler);
        map.insert("cp", crate::commands::cp::handle_argv as Handler);
        map.insert("mv", crate::commands::mv::handle_argv as Handler);
        map.insert("info", crate::commands::info::handle_argv as Handler);
        map.insert("incp", crate::commands::incp::handle_argv as Handler);
        map.insert("outcp", crate::commands::outcp::handle_argv as Handler);
        map.insert("load", crate::commands::load::handle_argv as Handler);
        map.insert("format", crate::commands::format::handle_argv as Handler);
        map.insert("statfs", crate::commands::statfs::handle_argv as Handler);
        map.insert("slink", crate::commands::slink::handle_argv as Handler);
        map.insert("rmslink", crate::commands::rmslink::handle_argv as Handler);
        map.insert("clear", crate::commands::clear::handle_argv as Handler);

        Self { map }
    }

    pub fn dispatch(&self, name: &str, argv: &[&str], context: &mut Context) {
        // Snapshot current working directory (if filesystem is open)
        let saved_cwd = match context.fs_mut() {
            Ok(fs) => Some(fs.pwd().to_string()),
            Err(_) => None,
        };

        if let Some(handler) = self.map.get(name) {
            handler(argv, context);
        } else {
            eprintln!("Unknown command: {name}");
        }

        // Restore original working directory (best-effort), except for "cd"

        if name != "cd" {
            if let Some(path) = saved_cwd {
                if let Ok(fs) = context.fs_mut() {
                    let _ = fs.cd(&path);
                }
            }
        }
    }
}

pub mod cat;
pub mod cd;
pub mod clear;
pub mod cp;
pub mod exit;
pub mod format;
pub mod incp;
pub mod info;
pub mod load;
pub mod ls;
pub mod mkdir;
pub mod mv;
pub mod outcp;
pub mod pwd;
pub mod rm;
pub mod rmdir;
pub mod rmslink;
pub mod slink;
pub mod statfs;
