//! `clear` command.
//!
//! Purpose:
//!   Clears the terminal screen so the user gets a fresh view.
//!
//! Usage:
//!   clear
//!
//! Output:
//!   (No textual output; screen is cleared)
//!
//! Notes:
//! - Uses ANSI escape sequences (widely supported on modern terminals,
//!   including Windows 10+ with virtual terminal processing enabled).
//! - If a terminal does not support ANSI, the escape codes may show
//!   as raw characters; optional future improvement could add
//!   platform-specific fallbacks (e.g. invoking `cls` or `clear`).
//! - Command ignores any extra arguments.
//!
//! Behavior:
//! - Sends ESC[2J (erase entire screen) and ESC[H (move cursor to home).
//! - Does not change filesystem state.
//!
//! Possible extensions:
//! - Reprint greeting header after clearing.
//! - Add an option `clear -g` to force greeting reprint.
//!
use crate::context::Context;

/// Handler for the `clear` command.
pub fn handle_argv(_argv: &[&str], _context: &mut Context) {
    // ANSI escape sequence to clear screen & move cursor to 0,0
    // \x1B == ESC
    print!("\x1B[2J\x1B[H");

    // Flush stdout to ensure the escape codes take effect immediately.
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    #[test]
    fn clear_runs_without_fs() {
        let mut ctx = Context::new();
        // Should not panic.
        handle_argv(&[], &mut ctx);
    }

    #[test]
    fn clear_ignores_extra_args() {
        let mut ctx = Context::new();
        handle_argv(&["unexpected"], &mut ctx);
    }
}
