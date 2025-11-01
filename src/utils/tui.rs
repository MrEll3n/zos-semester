use std::io;
use colored::Colorize;
use crate::commands::{Registry, Context};

fn write_greet() {
    println!("{}'s File System - {}", "MrEll3n".green(), env!("CARGO_PKG_VERSION").yellow());
}

fn write_prefix() {
    eprint!("> ");
}

pub fn handle_app_loop() {
    let stdin = io::stdin();
    let registry = Registry::new();
    let mut context = Context {};

    write_greet();
    loop {
        // before input
        write_prefix();
        let mut user_input = String::new();

        // user input
        let _ = stdin.read_line(&mut user_input);
        
        // preparing user input to be formated into 'command' and 'args'
        let trimmed = user_input.trim();
        if trimmed.is_empty() {
            continue
        }
        let mut it = trimmed.split_whitespace();
        let command = it.next().unwrap();
        let args: Vec<&str> = it.collect();

        // command dispatch
        registry.dispatch(command, &args, &mut context);
    }
}
