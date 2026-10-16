use crate::network::other::Session;
use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct ConnectionClosedPacket {
    pub user_identifier: u8,
}
impl Packet for ConnectionClosedPacket {
    const MAGIC: u8 = super::types::CONNECTION_CLOSED;

    fn serialize(&self, writer: &mut PacketWriter) {
        if self.user_identifier == 0b000000 {
            panic!("Invalid user identifier");
        }

        writer.write_sextet(self.user_identifier);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let user_id = reader.read_sextet();

        Ok(Self {
            user_identifier: user_id,
        })
    }
}

impl ConnectionClosedPacket {
    /// Creates a closing packet to close the given session
    pub fn close_session(session: Session) -> Self {
        Self {
            user_identifier: session.user_id,
        }
    }
}
