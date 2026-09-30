use std::cmp::min;
use std::io;
use std::net::UdpSocket;
use crate::packet_types::PacketType;

/// Sends a connection packet
pub fn send_connection(socket: &UdpSocket, username: &str) -> io::Result<usize> {
    let username_bytes: &[u8] = username.as_bytes();
    let mut length: usize = username.len();
    let mut content: [u8; 16] = [0; 16];

    if length == 0 {
        panic!("Username is empty.");
    }

    length = min(length, content.len());

    for i in 0..length {
        content[i] = username_bytes[i];
    }

    let mut buffer: [u8; 1024] = [0; 1024];

    buffer[0] = PacketType::Connection as u8;
    buffer[1] = (((length - 1) as u8 & 0b1111) << 4) | (content[0] >> 4);

    let mut index = 2;

    for i in 0..length {
        buffer[index] = (content[i] & 0b1111) << 4 | (content[i + 1] >> 4);
        index += 1;
    }

    buffer[index] = content[length - 1] & 0b1111 << 4;

    socket.send(&buffer)
}
