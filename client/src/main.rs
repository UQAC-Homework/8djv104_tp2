mod network;
mod time;

use crate::time::repeater::Repeater;
use network::packets;
use std::env;
use std::io;
use std::io::Write;
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use time::timeout::Timeout;

fn print_incoming_messages(socket: &UdpSocket) {
    let buffer: &mut [u8] = &mut [0; 1024];

    loop {
        // Read from socket
        let bytes_count = socket.recv(buffer);

        if bytes_count.is_err() {
            continue;
        }
        // Cut buffer

        println!("{}", String::from_utf8_lossy(buffer));
    }
}

fn main() {
    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let args_len = args.len();

    let mut username: &str = "Player";
    let mut server_ip: &str = "127.0.0.1";
    let mut server_port: u16 = 9999;

    if args_len > 1 {
        username = args[1].as_str();
    }

    if args_len > 2 {
        server_ip = args[2].as_str();
    }

    if args_len > 3 {
        server_port = args[3].parse::<u16>().expect("Invalid port number.");
    }

    // Create Socket
    let socket =
        UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).expect("Failed to bind to address.");

    let server_address = format!("{}:{}", server_ip, server_port);
    socket
        .connect(server_address)
        .expect("Failed to connect to server.");

    // Send username as first message
    packets::send_connection(&socket, username).expect("Failed to send username.");
    socket
        .set_nonblocking(true)
        .expect("Failed to set nonblocking.");

    let timer: Timeout = Timeout::new(Duration::from_secs(10));
    let mut repeater: Repeater = Repeater::new(Duration::from_secs(2));

    while !timer.has_expired() {
        
        if repeater.is_due() {
            repeater.reset();

            // Send
            println!("Trigger");
            packets::send_connection(&socket, username);
        }

        let a = socket.recv(&mut [0; 1024]);

        if a.is_ok() {
            println!("{}", a.ok().unwrap());
            break;
        }
    }

    if !timer.has_expired() {
        println!("You have seen this username.");
    } else {
        println!("You have timed out.");
    }

    return;
    let socket = Arc::new(socket);
    let recv_socket = Arc::clone(&socket);

    let thread_receive_handle = thread::spawn(move || {
        print_incoming_messages(&recv_socket);
    });

    loop {
        // Get user message
        print!("> ");

        io::stdout().flush().unwrap();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line.");

        input = input.trim().into();

        // Send message to server
        socket
            .send(input.as_bytes())
            .expect("Failed to send message to server.");

        // If requested exit, leave
        if input == "/quit" || input == "/exit" {
            break;
        }
    }

    // Close socket
    // (Handled by Rust)

    thread_receive_handle.join().expect("TODO: panic message");
}
