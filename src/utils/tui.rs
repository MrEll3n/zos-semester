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
    let registry = Registry::new();
    let mut context = Context {};


    write_greet();
    let stdin = io::stdin();
    loop {
        write_prefix();
        let mut user_input = String::new();
        let _ = stdin.read_line(&mut user_input);
        let trimmed = user_input.trim();

        if trimmed.is_empty() {
            continue
        }

        let mut it = trimmed.split_whitespace();
        let command = it.next().unwrap();
        let args: Vec<&str> = it.collect();


        registry.dispatch(command, &args, &mut context);
    }
}
