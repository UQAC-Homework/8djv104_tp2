use crate::network::packet::FieldError;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

#[derive(Debug, Clone)]
pub struct MessageRequestPacket {
    pub user_identifier: u8,
    pub last_message_identifier: u8,
    pub received_messages: u64,
}

impl Packet for MessageRequestPacket {
    const MAGIC: u8 = super::types::MESSAGE_REQUEST;

    fn serialize(&self, writer: &mut PacketWriter) {
        writer.write_sextet(self.user_identifier);
        writer.write_byte(self.last_message_identifier);
        writer.write_quadword(self.received_messages);
    }

    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError> {
        let user_id = reader.read_sextet();
        let last_message_identifier = reader.read_byte();
        let received_messages = reader.read_quadword();

        Ok(Self {
            user_identifier: user_id,
            last_message_identifier,
            received_messages,
        })
    }
}
