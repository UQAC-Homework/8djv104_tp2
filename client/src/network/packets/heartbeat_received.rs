use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct HeartbeatReceivedPacket {
    pub message_history: MessageHistory,
}
impl Packet for HeartbeatReceivedPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_RECEIVED;

    fn serialize(&self, _writer: &mut PacketWriter) {
        panic!("Clients are not allowed to send heartbeat receipts.")
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let message_history = MessageHistory::deserialize(reader)?;

        Ok(Self { message_history })
    }
}
