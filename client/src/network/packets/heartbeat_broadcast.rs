use crate::models::message_history::MessageHistory;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct HeartbeatBroadcastPacket {
    pub user_identifier: u8,
    pub message_history: MessageHistory,
}
impl Packet for HeartbeatBroadcastPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_BROADCAST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_sextet(self.user_identifier);
        self.message_history.serialize(writer);
    }

    fn deserialize(_reader: &mut PacketReader) -> Result<Self, FieldError> {
        panic!("Clients are not allowed to handle heartbeat broadcasts.")
    }
}
