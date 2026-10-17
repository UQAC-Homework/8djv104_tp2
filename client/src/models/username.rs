use crate::network::packet::FieldError;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;
use std::fmt;

/// Maximum length of a username in bytes
const USERNAME_MAX_LENGTH: usize = 16;

/// Structure used to define the name of a user
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username {
    content: String,
}

impl Username {
    pub fn new(name: impl Into<String>) -> Result<Self, FieldError> {
        let name = name.into();

        if name.is_empty() {
            return Err(FieldError::Empty);
        }

        if !name.is_ascii() {
            return Err(FieldError::NotAscii);
        }

        if name.len() > USERNAME_MAX_LENGTH {
            return Err(FieldError::TooLong {
                max: USERNAME_MAX_LENGTH,
            });
        }

        Ok(Self { content: name })
    }

    /// Serializes this username into the given writer
    pub fn serialize(&self, writer: &mut PacketWriter) {
        let length = self.content.len();

        writer.write_nybble((length - 1) as u8);
        writer.write_string(&self.content);
    }

    /// Deserializes this username into the given writer
    pub fn deserialize(reader: &mut PacketReader) -> Result<Username, FieldError> {
        let length_minus_one = reader.read_nybble()? as usize;
        let length = length_minus_one + 1;

        let raw_username = reader.read_string(length)?;
        Username::new(raw_username)
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.content)
    }
}
