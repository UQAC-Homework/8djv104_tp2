//! Wire format of the chat protocol (RFC sections "Headers" and "Packets").
//!
//! Every packet is a bit-packed record, most significant bit first
//! ("big-endian"). Fields are generally *not* byte aligned (user identifiers
//! are 6 bits, message lengths 10 bits, ...) and the variable-length
//! `Username` / `Message` fields shift everything that follows them, so the
//! codec works on a bit cursor instead of fixed byte offsets. The last byte
//! of a packet is padded with zero bits.

use std::fmt;

/// Longest message, in bytes (RFC: `Message Length` MUST NOT exceed 990).
pub const MAX_MESSAGE_LEN: usize = 990;

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

pub(crate) fn check_text(s: &str, max: usize) -> Result<(), FieldError> {
    if s.is_empty() {
        Err(FieldError::Empty)
    } else if !s.is_ascii() {
        Err(FieldError::NotAscii)
    } else if s.len() > max {
        Err(FieldError::TooLong { max })
    } else {
        Ok(())
    }
}

/// A chat message: 1 to 990 ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageText(String);

impl MessageText {
    pub fn new(text: impl Into<String>) -> Result<Self, FieldError> {
        let text = text.into();
        check_text(&text, MAX_MESSAGE_LEN)?;
        Ok(Self(text))
    }
}

impl fmt::Display for MessageText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
