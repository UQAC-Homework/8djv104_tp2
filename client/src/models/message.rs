use crate::network::packet::FieldError;
use crate::network::serialization::packet_writer::PacketWriter;

/// Maximum length of a message in bytes
const MESSAGE_MAX_LENGTH: usize = 990;

/// Structure used to define a message by a user
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    content: String,
}

impl Message {
    fn new(content: String) -> Result<Self, FieldError> {
        if content.is_empty() {
            return Err(FieldError::Empty);
        }

        if !content.is_ascii() {
            return Err(FieldError::NotAscii);
        }

        if content.len() > MESSAGE_MAX_LENGTH {
            return Err(FieldError::TooLong {
                max: MESSAGE_MAX_LENGTH,
            });
        }

        Ok(Self { content })
    }

    /// Creates a message from the given string
    pub fn from_string(content: &str) -> Result<Self, FieldError> {
        Self::new(content.to_string())
    }

    /// Serializes this message into the given writer
    pub fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_declet(self.content.len() as u16);
        writer.write_string(&self.content);
    }
}
