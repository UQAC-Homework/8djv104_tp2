use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct ConnectionRejectedPacket {}
impl Packet for ConnectionRejectedPacket {
    const MAGIC: u8 = super::types::CONNECTION_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        panic!("Clients are not allowed to reject connections.");
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        reader.read_byte();

        Self {}
    }
}
