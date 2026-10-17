use std::fmt;

/// List of every reason why a [`Packet`] has failed deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Type of the packet is disallowed
    ReservedType,

    /// No implementation was found for this type
    UnknownType(u8),

    /// Not enough data is available to decode the given type
    NotEnoughData(usize),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::ReservedType => write!(f, "Packet type 0x00 is reserved"),
            DecodeError::UnknownType(t) => write!(f, "Unsupported packet type {t:#04x}"),
            DecodeError::NotEnoughData(c) => write!(f, "Not enough data to read {c} bits"),
        }
    }
}

impl std::error::Error for DecodeError {}
