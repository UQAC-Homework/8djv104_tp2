use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::constants::MESSAGE_CONTENT_MAX_LENGTH;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct MessageSubmitPacket {
    pub user_identifier: u8,
    pub message_history: MessageHistory,
    pub message_identifier: u8,
    pub message: String,
}

impl Packet for MessageSubmitPacket {
    const MAGIC: u8 = super::types::MESSAGE_SUBMIT;

    fn serialize(&self, writer: &mut PacketWriter) {
        let length = self.message.len();

        if length == 0 {
            panic!("Can't submit an empty message");
        }

        if length > MESSAGE_CONTENT_MAX_LENGTH as usize {
            panic!("Message cannot be more than {MESSAGE_CONTENT_MAX_LENGTH} characters");
        }

        writer.write_sextet(self.user_identifier);
        self.message_history.serialize(writer);
        writer.write_byte(self.message_identifier);
        writer.write_declet(length as u16);
        writer.write_string(&self.message);
    }

    fn deserialize(_reader: &mut PacketReader) -> Result<Self, FieldError> {
        panic!("Clients cannot handle message submissions.");
    }
}
