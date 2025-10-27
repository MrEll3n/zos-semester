use std::{env, process};

fn handle_help(exit: bool) {
    println!("Usage: elfs-emu [--help] <filesystem.elfs>");
    if exit {
        process::exit(1);
    }
}

pub fn handle_prog_args() {
    let mut it = env::args().skip(1).peekable();

    if it.peek().is_none() {
        handle_help(true);
    }

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" => handle_help(true),
            _ => handle_help(true),
        }
    }
}
