//! Command-line chat client.
//!
//! All protocol logic lives in the library; this file only moves bytes
//! between the UDP socket, the terminal and [`Client`], and sleeps until the
//! client's next timer.

use network::other::DisconnectReason;
use rand::random_range;
use std::collections::VecDeque;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

mod application;
mod models;
mod network;
mod time;

use crate::application::arguments::Arguments;
use crate::application::events::event::UserEvent;
use crate::application::logger;
use crate::application::logger::Level;
use crate::models::message::Message;
use crate::models::username::Username;
use crate::network::client::client::Client;
use crate::network::packets::packet::decode_packet;

const USAGE: &str = "\
Usage: client <username> <server_ip> <server_port> [--loss P] [--verbose]

    <username>   : Name of the user to connect as
    <server_ip>  : IP address to connect to
    <server_port>: Port number to connect to

    [loss]   : Chance that the client voluntarily loses a packet
    [verbose]: Enable the logging of network packets 
";

/// How long to wait for outstanding acknowledgements after end of input.
const DRAIN_LIMIT: Duration = Duration::from_secs(3);

/// Everything the event loop can be woken up by.
enum Input {
    Datagram(Vec<u8>),
    Line(String),
    StdinClosed,
    SocketError(io::Error),
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = match Arguments::new(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(msg) => {
            if msg.is_some() {
                log_error!("Error while parsing arguments: {}\n", msg.unwrap());
                std::process::exit(1);
            }

            log_error!("{USAGE}");
            std::process::exit(2);
        }
    };

    if arguments.verbose {
        logger::set_level(Level::Debug);
    }

    let username =
        Username::new(arguments.username).map_err(|e| format!("Invalid username: {e}"))?;

    let full_address = format!("{}:{}", arguments.server_ip, arguments.server_port);
    let server = full_address
        .to_socket_addrs()
        .map_err(|e| {
            format!(
                "Cannot use `{}` as a server address ({e}); expected host:port",
                full_address
            )
        })?
        .next()
        .ok_or("could not resolve the server address")?;

    match run(username, server, arguments.packet_drop_rate) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            log_error!("Error while running the client: {e}");
            std::process::exit(2);
        }
    }
}

fn run(
    username: Username,
    address: SocketAddr,
    packet_drop_rate: f32,
) -> Result<i32, Box<dyn Error>> {
    let bind_address = match address.is_ipv4() {
        true => "0.0.0.0:0",
        false => "[::]:0",
    };

    let socket = UdpSocket::bind(bind_address)?;

    socket.connect(address)?;

    let (tx, rx) = mpsc::channel();
    spawn_socket_reader(socket.try_clone()?, tx.clone());
    spawn_stdin_reader(tx);

    let client_identifier = random_client_id();
    let mut client = Client::new(username, client_identifier);
    client.connect();

    let mut exit_code = 0;
    let mut typed_before_online: VecDeque<Message> = VecDeque::new();
    let mut input_closed = false;
    let mut drain_deadline: Option<Instant> = None;

    loop {
        let now = Instant::now();

        // Lines typed while the connection was still being established.
        while client.is_online() {
            let Some(message) = typed_before_online.pop_front() else {
                break;
            };
            let _ = client.send_message(message);
        }

        while let Some(event) = client.event_queue.pull() {
            match show(&event)? {
                Some(code) => {
                    exit_code = code;
                }
                None => {}
            }
        }
        flush_transmits(&mut client, &socket, packet_drop_rate)?;
        if client.is_offline() {
            break;
        }

        // End of input: leave once the server has everything we sent.
        if input_closed && client.is_online() && typed_before_online.is_empty() {
            let deadline = *drain_deadline.get_or_insert(now + DRAIN_LIMIT);

            if !client.has_unacknowledged() || now >= deadline {
                client.disconnect();
                continue;
            }
        }

        let mut wake = client.poll_timeout();

        match drain_deadline {
            Some(deadline) => {
                wake = Some(wake.map_or(deadline, |w| w.min(deadline)));
            }
            None => {}
        }

        let input = match wake {
            Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        match input {
            Ok(Input::Datagram(data)) => {
                match decode_packet(&data) {
                    Ok(packet) => log_debug!("<- {packet:?}"),
                    Err(e) => log_debug!("<- discarded: {e}"),
                }

                client.handle_packet(&data);
            }
            Ok(Input::Line(line)) => {
                let line = line.trim_end_matches('\r');
                if line == "/quit" {
                    client.disconnect();
                } else if !line.is_empty() {
                    match Message::from_string(line) {
                        Ok(message) => typed_before_online.push_back(message),
                        Err(e) => log_error!("Failed to send the message: {e}"),
                    }
                }
            }
            Ok(Input::StdinClosed) => input_closed = true,
            Ok(Input::SocketError(e)) => return Err(e.into()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        client.handle_timeout();
    }
    Ok(exit_code)
}

/// Sends everything the client queued. ICMP "connection refused" errors
/// surfacing on a connected UDP socket just mean the datagram was lost.
fn flush_transmits(
    client: &mut Client,
    socket: &UdpSocket,
    packet_drop_rate: f32,
) -> io::Result<()> {
    while let Some(datagram) = client.packet_queue.pull() {
        //if let Ok(packet) = decode_packet(&datagram) {
        //log_error!("-> {packet:?}");
        log_debug!("-> {datagram:?}");
        //}

        if random_range(0.0..1.0) <= packet_drop_rate {
            log_debug!("-X Packet was dropped.");
            continue;
        }

        match socket.send(&datagram) {
            Ok(_) => {}
            Err(e) => {
                if !matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionRefused | io::ErrorKind::WouldBlock
                ) {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}

/// Prints an event. Returns an exit code for events that end the program.
fn show(event: &UserEvent) -> io::Result<Option<i32>> {
    let mut out = io::stdout().lock();

    if matches!(event, UserEvent::Message { .. }) {
        writeln!(out, "{}", event)?;
        out.flush()?;
        return Ok(None);
    }

    log_info!("* {}", event);

    let code = match event {
        UserEvent::Connected => None,
        UserEvent::Rejected => Some(1),
        UserEvent::Timeout => Some(1),
        UserEvent::Disconnected(DisconnectReason::UserRequested) => Some(0),
        UserEvent::Disconnected(DisconnectReason::ServerClosed) => Some(1),
        UserEvent::Disconnected(DisconnectReason::ServerUnreachable) => Some(1),
        _ => None,
    };

    Ok(code)
}

fn spawn_socket_reader(socket: UdpSocket, tx: Sender<Input>) {
    thread::spawn(move || {
        let mut buf = [0u8; 2048];
        loop {
            match socket.recv(&mut buf) {
                Ok(n) => {
                    let _ = tx.send(Input::Datagram(buf[..n].to_vec()));
                }
                Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => continue,
                Err(e) => {
                    let _ = tx.send(Input::SocketError(e));
                    return;
                }
            }
        }
    });
}

fn spawn_stdin_reader(tx: Sender<Input>) {
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    let _ = tx.send(Input::Line(line));
                }
                Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                    log_error!("! line skipped: input is not valid text");
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(Input::StdinClosed);
    });
}

fn random_client_id() -> u8 {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    // `RandomState` is seeded randomly per process; four bits are enough.
    (RandomState::new().hash_one(std::process::id()) & 0x0F) as u8
}
