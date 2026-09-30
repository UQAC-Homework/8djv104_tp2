use std::env;
use std::io;
use std::io::Write;
use std::net::UdpSocket;

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
    socket
        .send(username.as_bytes())
        .expect("Failed to send username.");

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
}
