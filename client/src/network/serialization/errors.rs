use std::fmt;
use crate::network::packet::FieldError;

/// List of every reason why a [`Packet`] has failed deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Packet ended before all fields were read
    Truncated,

    /// Packet ended with data left to read
    TrailingBytes,

    /// Type of the packet is disallowed
    ReservedType,

    /// No implementation was found for this type
    UnknownType(u8),

    /// Field was invalid
    InvalidField(FieldError),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "truncated packet"),
            DecodeError::TrailingBytes => write!(f, "unexpected bytes after the last field"),
            DecodeError::ReservedType => write!(f, "packet type 0x00 is reserved"),
            DecodeError::UnknownType(t) => write!(f, "unsupported packet type {t:#04x}"),
            DecodeError::InvalidField(what) => write!(f, "invalid field {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<FieldError> for DecodeError {
    fn from(err: FieldError) -> Self {
        DecodeError::InvalidField(err)
    }
}
