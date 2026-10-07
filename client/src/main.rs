mod network;
mod time;

use std::net::UdpSocket;

fn main() {
    // Create Socket
    let socket =
        UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).expect("Failed to bind to address.");

    let server_address = format!("{}:{}", "127.0.0.1", 9999);
    socket
        .connect(server_address)
        .expect("Failed to connect to server.");

    // Close socket
    // (Handled by Rust)
}
