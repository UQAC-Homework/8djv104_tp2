use crate::models::message::Message;
use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct MessageSubmitPacket {
    pub user_identifier: u8,
    pub message_history: MessageHistory,
    pub message_identifier: u8,
    pub message: Message,
}

impl Packet for MessageSubmitPacket {
    const MAGIC: u8 = super::types::MESSAGE_SUBMIT;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_sextet(self.user_identifier);
        self.message_history.serialize(writer);
        writer.write_byte(self.message_identifier);
        self.message.serialize(writer);
    }

    fn deserialize(_reader: &mut PacketReader) -> Result<Self, FieldError> {
        panic!("Clients cannot handle message submissions.");
    }
}
