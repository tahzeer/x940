use std::fmt;

/// Error types for the x940 MT940 parser.
#[derive(Debug)]
pub enum ParseError {
    /// The input is empty or contains no MT940 data
    EmptyInput,

    /// A mandatory tag (e.g., `:20:`, `:25:`, `:60F:`, `:62F:`) is missing
    MissingTag { tag: &'static str, context: String },

    /// A tag expected by the current FSM state appeared in the wrong order
    UnexpectedTag { tag: String, expected_state: String },

    /// Could not parse a date field (expected YYMMDD format)
    InvalidDate { value: String, tag: &'static str },

    /// Could not parse an amount field (expected comma-decimal format)
    InvalidAmount { value: String, tag: &'static str },

    /// A tag value does not match the expected SWIFT format
    InvalidFormat { tag: &'static str, value: String, reason: String },

    /// Underlying I/O or string parsing error
    Parse { message: String },

    /// A duplicate mandatory tag was found when only one is allowed
    DuplicateTag { tag: &'static str },
}

pub type Result<T> = std::result::Result<T, ParseError>;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "Input contains no MT940 data"),
            Self::MissingTag { tag, context } => {
                write!(f, "Missing mandatory tag {} ({})", tag, context)
            }
            Self::UnexpectedTag { tag, expected_state } => {
                write!(f, "Unexpected tag {} in state {}", tag, expected_state)
            }
            Self::InvalidDate { value, tag } => {
                write!(f, "Invalid date '{}' in tag {}", value, tag)
            }
            Self::InvalidAmount { value, tag } => {
                write!(f, "Invalid amount '{}' in tag {}", value, tag)
            }
            Self::InvalidFormat { tag, value, reason } => {
                write!(f, "Invalid format in tag {}: '{}': {}", tag, value, reason)
            }
            Self::Parse { message } => {
                write!(f, "Parse error: {}", message)
            }
            Self::DuplicateTag { tag } => {
                write!(f, "Duplicate mandatory tag {}", tag)
            }
        }
    }
}

impl std::error::Error for ParseError {}
