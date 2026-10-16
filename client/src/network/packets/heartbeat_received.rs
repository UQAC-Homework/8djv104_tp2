use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct HeartbeatReceivedPacket {
    pub last_message_identifier: u8,
    pub received_messages: u64,
}
impl Packet for HeartbeatReceivedPacket {
    const MAGIC: u8 = super::types::HEARTBEAT_RECEIVED;

    fn serialize(&self, writer: &mut PacketWriter) {
        panic!("Clients are not allowed to send heartbeat receipts.")
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let last_message_identifier = reader.read_byte();
        let received_messages = reader.read_quadword();

        Ok(Self {
            last_message_identifier,
            received_messages,
        })
    }
}
