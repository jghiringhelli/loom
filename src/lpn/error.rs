/// LPN error types.
use std::fmt;

/// An error that can occur during LPN parsing or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpnError {
    /// A line could not be parsed as a valid LPN instruction.
    Parse { line: usize, message: String },
    /// An instruction was parsed but failed during execution.
    Execute { instruction: String, cause: String },
    /// An I/O error occurred while reading/writing a file.
    Io { path: String, cause: String },
}

impl fmt::Display for LpnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { line, message } => write!(f, "parse error at line {line}: {message}"),
            Self::Execute { instruction, cause } => {
                write!(f, "execute error for `{instruction}`: {cause}")
            }
            Self::Io { path, cause } => write!(f, "I/O error on `{path}`: {cause}"),
        }
    }
}
