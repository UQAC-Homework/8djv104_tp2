use crate::network::packet::FieldError;
use crate::network::packets::connection_accepted::ConnectionAcceptedPacket;
use crate::network::packets::connection_closed::ConnectionClosedPacket;
use crate::network::packets::connection_rejected::ConnectionRejectedPacket;
use crate::network::packets::connection_request::ConnectionRequestPacket;
use crate::network::packets::heartbeat_broadcast::HeartbeatBroadcastPacket;
use crate::network::packets::heartbeat_received::HeartbeatReceivedPacket;
use crate::network::packets::heartbeat_request::HeartbeatRequestPacket;
use crate::network::packets::message_broadcast::MessageBroadcastPacket;
use crate::network::packets::message_request::MessageRequestPacket;
use crate::network::packets::message_submit::MessageSubmitPacket;
use crate::network::serialization::errors::DecodeError;
use crate::network::serialization::packet_reader::PacketReader;
use crate::network::serialization::packet_writer::PacketWriter;

/// Header that defines the type of the packet
pub trait Packet: Sized {
    const MAGIC: u8;

    /// Serializes this packet into the given writer
    fn serialize(&self, writer: &mut PacketWriter);

    /// Deserializes this packet from the given reader
    fn deserialize(reader: &mut PacketReader) -> Result<Self, FieldError>;
}

/// Encodes the given packet into a binary format
pub fn encode_packet<P: Packet>(packet: &P) -> Vec<u8> {
    let mut writer = PacketWriter::new();

    writer.write_byte(P::MAGIC);
    packet.serialize(&mut writer);

    writer.buffer().to_vec()
}

pub fn decode_packet(datagram: &[u8]) -> Result<NetworkPacket, DecodeError> {
    let mut reader = PacketReader::new(datagram);

    let packet_type = reader.read_byte();

    let packet = match packet_type {
        0x00 => return Err(DecodeError::ReservedType),

        ConnectionAcceptedPacket::MAGIC => {
            NetworkPacket::ConnectionAccepted(ConnectionAcceptedPacket::deserialize(&mut reader)?)
        }
        ConnectionClosedPacket::MAGIC => {
            NetworkPacket::ConnectionClosed(ConnectionClosedPacket::deserialize(&mut reader)?)
        }
        ConnectionRejectedPacket::MAGIC => {
            NetworkPacket::ConnectionRejected(ConnectionRejectedPacket::deserialize(&mut reader)?)
        }
        ConnectionRequestPacket::MAGIC => {
            NetworkPacket::ConnectionRequest(ConnectionRequestPacket::deserialize(&mut reader)?)
        }
        HeartbeatBroadcastPacket::MAGIC => {
            NetworkPacket::HeartbeatBroadcast(HeartbeatBroadcastPacket::deserialize(&mut reader)?)
        }
        HeartbeatReceivedPacket::MAGIC => {
            NetworkPacket::HeartbeatReceived(HeartbeatReceivedPacket::deserialize(&mut reader)?)
        }
        HeartbeatRequestPacket::MAGIC => {
            NetworkPacket::HeartbeatRequest(HeartbeatRequestPacket::deserialize(&mut reader)?)
        }
        MessageBroadcastPacket::MAGIC => {
            NetworkPacket::MessageBroadcast(MessageBroadcastPacket::deserialize(&mut reader)?)
        }
        MessageRequestPacket::MAGIC => {
            NetworkPacket::MessageRequest(MessageRequestPacket::deserialize(&mut reader)?)
        }
        MessageSubmitPacket::MAGIC => {
            NetworkPacket::MessageSubmit(MessageSubmitPacket::deserialize(&mut reader)?)
        }

        other => return Err(DecodeError::UnknownType(other)),
    };

    Ok(packet)
}

#[derive(Debug, Clone)]
pub enum NetworkPacket {
    ConnectionAccepted(ConnectionAcceptedPacket),
    ConnectionClosed(ConnectionClosedPacket),
    ConnectionRejected(ConnectionRejectedPacket),
    ConnectionRequest(ConnectionRequestPacket),
    HeartbeatBroadcast(HeartbeatBroadcastPacket),
    HeartbeatReceived(HeartbeatReceivedPacket),
    HeartbeatRequest(HeartbeatRequestPacket),
    MessageBroadcast(MessageBroadcastPacket),
    MessageRequest(MessageRequestPacket),
    MessageSubmit(MessageSubmitPacket),
}
