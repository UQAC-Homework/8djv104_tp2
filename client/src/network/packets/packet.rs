use crate::network::packet::DecodeError;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

/// Header that defines the type of the packet
pub trait Packet: Sized {
    const MAGIC: u8;

    /// Serializes this packet into the given writer
    fn serialize(&self, writer: &mut PacketWriter);

    /// Deserializes this packet from the given reader
    fn deserialize(reader: &mut PacketReader) -> Self;
}

pub fn encode_packet(packet: &impl Packet) -> Vec<u8> {
    let mut writer = PacketWriter::new();

    packet.serialize(&mut writer);

    writer.buffer().to_vec()
}

pub fn decode_packet<P: Packet>(datagram: &[u8]) -> Result<P, DecodeError> {
    let mut reader = PacketReader::new(datagram);

    let packet_type = reader.read_byte();

    if packet_type == 0x00 {
        return Err(DecodeError::ReservedType);
    }

    if packet_type != P::MAGIC {
        return Err(DecodeError::UnknownType(packet_type));
    }

    Ok(P::deserialize(&mut reader))
}
