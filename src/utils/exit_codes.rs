#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    Error = 1
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        code as i32
    }
}
