mod commands;
use commands::Context;
mod utils;
use utils::{arg_man, tui};

fn main() -> std::io::Result<()> {
    let mut context = Context::new();
    arg_man::handle_prog_args(&mut context);
    tui::handle_app_loop(&mut context);
    Ok(())
}
