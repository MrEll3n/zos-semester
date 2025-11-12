use crate::commands::Registry;
use crate::context::Context;
use colored::Colorize;
use std::io;

fn write_greet() {
    eprintln!(
        "{}'s File System - {}",
        "MrEll3n".green(),
        env!("CARGO_PKG_VERSION").yellow()
    );
}

fn write_prefix(context: &mut Context) {
    // Obtain current working directory (fallback to "/" if FS not opened)
    let path = match context.fs_mut() {
        Ok(fs) => fs.pwd().to_string(),
        Err(_) => "/".to_string(),
    };
    eprint!("{}> ", path.blue());
}

pub fn handle_app_loop(context: &mut Context) {
    let stdin = io::stdin();
    let registry = Registry::new();

    write_greet();
    loop {
        // before input
        write_prefix(context);
        let mut user_input = String::new();

        // user input
        let _ = stdin.read_line(&mut user_input);

        // preparing user input to be formated into 'command' and 'args'
        let trimmed = user_input.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut it = trimmed.split_whitespace();
        let command = it.next().unwrap();
        let args: Vec<&str> = it.collect();

        // command dispatch
        registry.dispatch(command, &args, context);
    }
}
