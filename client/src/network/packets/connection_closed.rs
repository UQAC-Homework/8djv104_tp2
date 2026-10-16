use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct ConnectionClosedPacket {
    pub user_identifier: u8,
}
impl Packet for ConnectionClosedPacket {
    const MAGIC: u8 = super::types::CONNECTION_CLOSED;

    fn serialize(&self, writer: &mut PacketWriter) {
        if self.user_identifier == 0b000000 {
            panic!("Invalid user identifier");
        }

        writer.write_byte(Self::MAGIC);
        writer.write_sextet(self.user_identifier);
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        reader.read_byte();

        let user_id = reader.read_sextet();

        Self {
            user_identifier: user_id,
        }
    }
}
