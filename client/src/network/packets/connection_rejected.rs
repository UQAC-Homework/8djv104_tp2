use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct ConnectionRejectedPacket {}
impl Packet for ConnectionRejectedPacket {
    const MAGIC: u8 = super::types::CONNECTION_REJECTED;

    fn serialize(&self, writer: &mut PacketWriter) {
        panic!("Clients are not allowed to reject connections.");
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        Ok(Self {})
    }
}
