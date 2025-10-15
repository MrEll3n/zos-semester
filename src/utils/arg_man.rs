use std::env;

const ALLOWED_ARGS: &[&str] = &["-h", "--help"];

fn get_arguments() -> Vec<String> {
    env::args().collect()
}

