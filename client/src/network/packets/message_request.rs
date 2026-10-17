use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct MessageRequestPacket {
    pub user_identifier: u8,
    pub message_history: MessageHistory,
}

impl Packet for MessageRequestPacket {
    const MAGIC: u8 = super::types::MESSAGE_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_sextet(self.user_identifier);
        self.message_history.serialize(writer);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let user_id = reader.read_sextet()?;
        let message_history = MessageHistory::deserialize(reader)?;

        Ok(Self {
            user_identifier: user_id,
            message_history,
        })
    }
}
