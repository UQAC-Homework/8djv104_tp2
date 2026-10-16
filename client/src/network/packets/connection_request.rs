use crate::models::username::Username;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct ConnectionRequestPacket {
    pub client_identifier: u8,
    pub username: Username,
}

impl Packet for ConnectionRequestPacket {
    const MAGIC: u8 = super::types::CONNECTION_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_nybble(self.client_identifier);
        self.username.serialize(writer);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let client_identifier = reader.read_nybble();
        let username = Username::deserialize(reader)?;

        Ok(Self {
            client_identifier,
            username,
        })
    }
}
