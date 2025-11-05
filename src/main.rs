mod commands;
mod context;
mod fs;
mod utils;

use context::Context;
use utils::{cli, tui};

fn main() -> std::io::Result<()> {
    let mut context = Context::new();
    cli::handle_prog_args(&mut context);
    tui::handle_app_loop(&mut context);
    Ok(())
}
