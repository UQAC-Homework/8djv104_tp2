use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct MessageBroadcastPacket {
    pub message_identifier: u8,
    pub message_history: MessageHistory,
    pub username: String,
    pub message: String,
}

impl Packet for MessageBroadcastPacket {
    const MAGIC: u8 = super::types::MESSAGE_BROADCAST;

    fn serialize(&self, _writer: &mut PacketWriter) {
        panic!("Clients cannot broadcast messages.");
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let message_id = reader.read_byte()?;
        let message_history = MessageHistory::deserialize(reader)?;
        let username_length = reader.read_nybble()? + 1;
        let username = reader.read_string(username_length as usize)?;
        let message_length = reader.read_declet()?;
        let message = reader.read_string(message_length as usize)?;

        Ok(Self {
            message_identifier: message_id,
            message_history,
            username,
            message,
        })
    }
}
