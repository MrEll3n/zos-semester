use std::io;
use colored::Colorize;

fn write_greet() {
    println!("{}'s File System - {}", "MrEll3n".green(), env!("CARGO_PKG_VERSION").yellow());
}

fn write_prefix() {
    eprint!("> ");
}

pub fn handle_app_loop() {
    write_greet();
    let stdin = io::stdin();
    loop {
        write_prefix();
        let mut user_input = String::new();
        let _ = stdin.read_line(&mut user_input);
        if user_input.trim_end() == "exit" {
            break
        }
    }
}
