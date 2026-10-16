use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct ConnectionAcceptedPacket {
    pub user_identifier: u8,
}

impl Packet for ConnectionAcceptedPacket {
    const MAGIC: u8 = super::types::CONNECTION_ACCEPTED;

    fn serialize(&self, writer: &mut PacketWriter) {
        panic!("Clients are not allowed to accept connections.");
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        reader.read_byte();

        let user_id = reader.read_sextet();

        Self {
            user_identifier: user_id,
        }
    }
}
