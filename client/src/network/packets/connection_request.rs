use crate::models::username::Username;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;
use std::cmp::min;

#[derive(Debug, Clone)]
pub struct ConnectionRequestPacket {
    pub client_identifier: u8,
    pub username: Username,
}

impl Packet for ConnectionRequestPacket {
    const MAGIC: u8 = super::types::CONNECTION_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        let username = self.username.to_string();
        let mut length: usize = username.len();

        if length == 0 {
            panic!("Username is empty.");
        }

        length = min(length, 16);

        writer.write_nybble(self.client_identifier);
        writer.write_nybble((length - 1) as u8);
        writer.write_string(&username);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let client_identifier = reader.read_nybble();
        let length_minus_one = reader.read_nybble() as usize;
        let length = length_minus_one + 1;

        let raw_username = reader.read_string(length);
        let username = Username::new(raw_username)?;

        Ok(Self {
            client_identifier,
            username,
        })
    }
}
