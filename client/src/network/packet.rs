//! Wire format of the chat protocol (RFC sections "Headers" and "Packets").
//!
//! Every packet is a bit-packed record, most significant bit first
//! ("big-endian"). Fields are generally *not* byte aligned (user identifiers
//! are 6 bits, message lengths 10 bits, ...) and the variable-length
//! `Username` / `Message` fields shift everything that follows them, so the
//! codec works on a bit cursor instead of fixed byte offsets. The last byte
//! of a packet is padded with zero bits.

use std::fmt;
use crate::network::serialization::errors::DecodeError;
// ---------------------------------------------------------------------------
// Validated field types
// ---------------------------------------------------------------------------

/// Why a username or message was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    Empty,
    NotAscii,
    TooLong { max: usize },
    BadEncoding(DecodeError),
}

impl From<DecodeError> for FieldError {
    fn from(value: DecodeError) -> Self {
        FieldError::BadEncoding(value)
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::Empty => write!(f, "Field must not be empty"),
            FieldError::NotAscii => write!(f, "Field must contain ASCII characters only"),
            FieldError::TooLong { max } => write!(f, "Field must be at most {max} characters"),
            FieldError::BadEncoding(e) => write!(f, "Field was encoded in an invalid encoding ({e})"),
        }
    }
}

impl std::error::Error for FieldError {}
