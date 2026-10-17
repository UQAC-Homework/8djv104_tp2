use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct ConnectionAcceptedPacket {
    pub user_identifier: u8,
}

impl Packet for ConnectionAcceptedPacket {
    const MAGIC: u8 = super::types::CONNECTION_ACCEPTED;

    fn serialize(&self, _writer: &mut PacketWriter) {
        panic!("Clients are not allowed to accept connections.");
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let user_id = reader.read_sextet()?;

        Ok(Self {
            user_identifier: user_id,
        })
    }
}
