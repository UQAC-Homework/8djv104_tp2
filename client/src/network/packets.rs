use crate::network::packet_types::PacketType;
use crate::network::packet_writer::PacketWriter;
use std::cmp::min;
use std::io;
use std::net::UdpSocket;

/// Sends a connection packet
pub fn send_connection(socket: &UdpSocket, username: &str) -> io::Result<usize> {
    let username_bytes: &[u8] = username.as_bytes();
    let mut length: usize = username.len();

    if length == 0 {
        panic!("Username is empty.");
    }

    length = min(length, 16);

    let mut writer: PacketWriter = PacketWriter::new();

    writer.write_byte(PacketType::Connection as u8);
    writer.write_nybble((length - 1) as u8);
    for i in 0..length {
        writer.write_byte(username_bytes[i]);
    }

    socket.send(writer.buffer())
}
