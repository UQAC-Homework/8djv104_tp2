use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct HeartbeatRequestPacket {
    pub message_history: MessageHistory,
}
impl Packet for HeartbeatRequestPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_REQUEST;

    fn serialize(&self, _writer: &mut PacketWriter) {
        panic!("Clients are not allowed to request heartbeats.")
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let message_history = MessageHistory::deserialize(reader)?;

        Ok(Self { message_history })
    }
}
