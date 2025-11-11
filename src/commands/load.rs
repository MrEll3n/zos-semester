use crate::commands::Registry;
use crate::context::Context;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// load s1
/// Assignment spec: Executes commands from a host file.
/// Outputs:
///   OK             - if file read and all commands processed
///   FILE NOT FOUND - if the file cannot be opened
///
/// Behavior:
/// - Reads the given host file line by line.
/// - Each non-empty line is treated like a command input (same parsing as interactive loop).
/// - Lines starting with '#' are treated as comments and skipped.
/// - Whitespace-only lines are skipped.
/// - Commands found inside the load file produce their normal outputs.
/// - After finishing the file, prints "OK" (unless the file could not be opened).
///
/// Safety considerations:
/// - No recursion guard (nested `load` calls can happen).
///   You can add a recursion depth guard later if needed.
///
/// Usage example inside interactive session:
///   > load script.txt
///   (commands from script execute...)
///   OK
pub fn handle_argv(argv: &[&str], context: &mut Context) {
    // Validate number of arguments (exactly one expected: path to host file)
    if argv.len() != 1 {
        println!("FILE NOT FOUND");
        return;
    }
    let host_path = argv[0];

    // Try opening the host file
    let file = match File::open(host_path) {
        Ok(f) => f,
        Err(_) => {
            println!("FILE NOT FOUND");
            return;
        }
    };

    // Prepare command registry (same handlers as interactive mode)
    let registry = Registry::new();
    let reader = BufReader::new(file);

    // Process each line
    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => {
                // If a read error occurs mid-way, stop further processing.
                // You could decide to print a different error; spec only defines FILE NOT FOUND vs OK.
                println!("FILE NOT FOUND");
                return;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue; // comment
        }

        // Split into command + args
        let mut it = trimmed.split_whitespace();
        let cmd = match it.next() {
            Some(c) => c,
            None => continue,
        };
        let args: Vec<&str> = it.collect();

        // Dispatch
        registry.dispatch(cmd, &args, context);
    }

    // Finished processing file
    println!("OK");
}
