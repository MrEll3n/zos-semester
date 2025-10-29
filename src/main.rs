mod utils;
use utils::{arg_man, tui};

fn main() -> std::io::Result<()> {
    arg_man::handle_prog_args();
    tui::handle_app_loop();
    Ok(())
}
