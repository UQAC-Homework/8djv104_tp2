use crate::network::packets::constants::MESSAGE_CONTENT_MAX_LENGTH;
use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct MessageSubmitPacket {
    pub user_identifier: u8,
    pub last_message_identifier: u8,
    pub received_messages: u64,
    pub message_identifier: u8,
    pub message: String,
}

impl Packet for MessageSubmitPacket {
    const MAGIC: u8 = super::types::MESSAGE_SUBMIT;

    fn serialize(&self, writer: &mut PacketWriter) {
        let length = self.message.len();

        if length == 0 {
            panic!("Can't submit an empty message");
        }

        if length > MESSAGE_CONTENT_MAX_LENGTH as usize {
            panic!("Message cannot be more than {MESSAGE_CONTENT_MAX_LENGTH} characters");
        }

        writer.write_byte(Self::MAGIC);
        writer.write_sextet(self.user_identifier);
        writer.write_byte(self.last_message_identifier);
        writer.write_quadword(self.received_messages);
        writer.write_byte(self.message_identifier);
        writer.write_declet(length as u16);
        writer.write_string(&self.message);
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        panic!("Clients cannot handle message submissions.");
    }
}
