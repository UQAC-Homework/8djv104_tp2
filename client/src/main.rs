use std::env;
use std::net;

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
        server_port = args[3].parse::<u16>().unwrap();
    }

    println!("Hello, world!");
    println!("{}", username);
    println!("{}", server_ip);
    println!("{}", server_port);
}
