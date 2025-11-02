use crate::commands::Context;
use crate::utils::file_man::handle_fs;
use std::{env, process};

fn handle_help(exit: bool) {
    println!("Usage: elfs-emu [--help] <filesystem.elfs>");
    if exit {
        process::exit(1);
    }
}

pub fn handle_prog_args(context: &mut Context) {
    let mut it = env::args().skip(1).peekable();

    if it.peek().is_none() {
        handle_help(true);
    }

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" => handle_help(true),
            fs_path => handle_fs(fs_path, context),
        }
    }
}
