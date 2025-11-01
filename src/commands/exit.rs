use super::Context;
use crate::utils::exit_codes::ExitCode;

pub fn handle_argv(argv: &[&str], context: &mut Context) {
    std::process::exit(ExitCode::Success.into())
}
