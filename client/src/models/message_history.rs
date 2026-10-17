use crate::network::packet::FieldError;
use crate::network::reliability::Arrival;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Clone, Debug, Copy)]
pub struct MessageHistory {
    last_message_identifier: u8,
    received_messages: u64,
}

impl MessageHistory {
    /// Empty message history
    pub const EMPTY: Self = Self {
        last_message_identifier: u8::MAX,
        received_messages: u64::MAX,
    };

    /// Serializes this history into the given writer
    pub fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_byte(self.last_message_identifier);
        writer.write_quadword(self.received_messages);
    }

    /// Deserializes a history from the given reader
    pub fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let last_message_identifier = reader.read_byte()?;
        let received_messages = reader.read_quadword()?;

        Ok(Self {
            last_message_identifier,
            received_messages,
        })
    }

    /// Checks if the given message identifier is ahead of this history
    pub fn is_head(&self, message_identifier: u8) -> bool {
        let distance = self
            .last_message_identifier
            .wrapping_sub(message_identifier);

        1 <= distance && distance <= 127
    }

    /// Checks if the given message identifier is missing
    pub fn is_missing(&self, message_identifier: u8) -> bool {
        let forward = message_identifier.wrapping_sub(self.last_message_identifier);

        match forward {
            0 => false,
            1..=128 => true,
            _ => {
                // Older than `last`: look up bit `last - id - 1`.
                let bit = 255 - forward;
                bit < 64 && (self.received_messages >> bit) & 1 == 0
            }
        }
    }

    /// Marks the given message identifier as found
    pub fn mark_as_found(&mut self, message_identifier: u8) -> Arrival {
        let ahead = message_identifier.wrapping_sub(self.last_message_identifier);

        if (1..=127).contains(&ahead) {
            // New message. Bit n describes `last - 1 - n`, so moving `last`
            // forward by `ahead` shifts the old bits up by `ahead`, marks
            // the old `last` as received (bit `ahead - 1`) and leaves the
            // identifiers in between as missing (0).
            let shifted = if ahead >= 64 {
                0
            } else {
                self.received_messages << ahead
            };

            let old_last = if ahead <= 64 { 1u64 << (ahead - 1) } else { 0 };
            self.last_message_identifier = message_identifier;
            self.received_messages = shifted | old_last;

            return Arrival::New { distance: ahead };
        }

        match self
            .last_message_identifier
            .wrapping_sub(message_identifier)
        {
            1..=64 => {
                let bit = 1u64
                    << (self
                    .last_message_identifier
                    .wrapping_sub(message_identifier)
                    - 1);
                if self.received_messages & bit != 0 {
                    Arrival::Ignored
                } else {
                    self.received_messages |= bit;
                    Arrival::Late
                }
            }
            // 0: duplicate of `last`; above 64: older than the window.
            _ => Arrival::Ignored,
        }
    }
}
