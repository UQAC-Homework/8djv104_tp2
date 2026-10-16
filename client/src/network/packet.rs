//! Wire format of the chat protocol (RFC sections "Headers" and "Packets").
//!
//! Every packet is a bit-packed record, most significant bit first
//! ("big-endian"). Fields are generally *not* byte aligned (user identifiers
//! are 6 bits, message lengths 10 bits, ...) and the variable-length
//! `Username` / `Message` fields shift everything that follows them, so the
//! codec works on a bit cursor instead of fixed byte offsets. The last byte
//! of a packet is padded with zero bits.

use std::fmt;

// ---------------------------------------------------------------------------
// Validated field types
// ---------------------------------------------------------------------------

/// Why a username or message was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    Empty,
    NotAscii,
    TooLong { max: usize },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::Empty => write!(f, "must not be empty"),
            FieldError::NotAscii => write!(f, "must contain ASCII characters only"),
            FieldError::TooLong { max } => write!(f, "must be at most {max} characters"),
        }
    }
}

impl std::error::Error for FieldError {}
