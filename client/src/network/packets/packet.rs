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
