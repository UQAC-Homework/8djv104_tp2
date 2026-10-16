use crate::network::packets::packet::Packet;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

pub struct MessageBroadcastPacket {
    pub message_identifier: u8,
    pub last_message_identifier: u8,
    pub received_messages: u64,
    pub username: String,
    pub message: String,
}

impl Packet for MessageBroadcastPacket {
    const MAGIC: u8 = super::types::MESSAGE_BROADCAST;

    fn serialize(&self, writer: &mut PacketWriter) {
        panic!("Clients cannot broadcast messages.");
    }

    fn deserialize(reader: &mut PacketReader) -> Self {
        reader.read_byte();

        let message_id = reader.read_byte();
        let last_message_id = reader.read_byte();
        let received_messages = reader.read_quadword();
        let username_length = reader.read_nybble() + 1;
        let username = reader.read_string(username_length as usize);
        let message_length = reader.read_nybble();
        let message = reader.read_string(message_length as usize);

        Self {
            message_identifier: message_id,
            last_message_identifier: last_message_id,
            received_messages,
            username,
            message,
        }
    }
}
