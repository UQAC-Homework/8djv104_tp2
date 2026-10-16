use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct HeartbeatBroadcastPacket {
    pub user_identifier: u8,
    pub last_message_identifier: u8,
    pub received_messages: u64,
}
impl Packet for HeartbeatBroadcastPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_BROADCAST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_sextet(self.user_identifier);
        writer.write_byte(self.last_message_identifier);
        writer.write_quadword(self.received_messages);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        panic!("Clients are not allowed to handle heartbeat broadcasts.")
    }
}
