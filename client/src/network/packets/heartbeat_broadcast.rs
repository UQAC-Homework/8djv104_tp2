use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct HeartbeatBroadcastPacket {
    pub user_identifier: u8,
    pub last_message_identifier: u8,
    pub received_messages: u64,
}
impl Packet for HeartbeatBroadcastPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_BROADCAST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_byte(Self::MAGIC);
        writer.write_sextet(self.user_identifier);
        writer.write_byte(self.last_message_identifier);
        writer.write_quadword(self.received_messages);
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        panic!("Clients are not allowed to handle heartbeat broadcasts.")
    }
}
