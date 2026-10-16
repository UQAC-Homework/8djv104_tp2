use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;
use std::cmp::min;

pub struct ConnectionRequestPacket {
    pub client_identifier: u8,
    pub username: String,
}

impl Packet for ConnectionRequestPacket {
    const MAGIC: u8 = super::types::CONNECTION_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        let mut length: usize = self.username.len();

        if length == 0 {
            panic!("Username is empty.");
        }

        length = min(length, 16);

        writer.write_byte(Self::MAGIC);
        writer.write_nybble(self.client_identifier);
        writer.write_nybble((length - 1) as u8);
        writer.write_string(&*self.username);
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        reader.read_byte();

        let client_identifier = reader.read_nybble();
        let length_minus_one = reader.read_nybble() as usize;
        let length = length_minus_one + 1;

        let username = reader.read_string(length);

        Self {
            client_identifier,
            username,
        }
    }
}
