//! Command-line chat client.
//!
//! All protocol logic lives in the library; this file only moves bytes
//! between the UDP socket, the terminal and [`Client`], and sleeps until the
//! client's next timer.

use network::other::DisconnectReason;
use std::collections::VecDeque;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{ToSocketAddrs, UdpSocket};
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
Usage: udp-chat-client [--verbose] <server:port> <username>

  <server:port>  address of the chat server, e.g. 127.0.0.1:9000
  <username>     1-16 ASCII characters
  -v, --verbose  log every packet sent and received to stderr

Type a line and press Enter to send it. `/quit` (or end of input) leaves the
chat. When input ends, the client first waits (up to 3 s) until the server has
acknowledged everything that was sent.";

/// How long to wait for outstanding acknowledgements after end of input.
const DRAIN_LIMIT: Duration = Duration::from_secs(3);

/// Everything the event loop can be woken up by.
enum Input {
    Datagram(Vec<u8>),
    Line(String),
    StdinClosed,
    SocketError(io::Error),
}

fn main() {
    let args = match Arguments::new(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(msg) => {
            if !msg.is_empty() {
                log_error!("error: {msg}\n");
            }
            log_error!("{USAGE}");
            std::process::exit(2);
        }
    };

    match run(args) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            log_error!("error: {e}");
            std::process::exit(2);
        }
    }
}

fn run(arguments: Arguments) -> Result<i32, Box<dyn Error>> {
    if arguments.verbose {
        logger::set_level(Level::Debug);
    }

    let username =
        Username::new(arguments.username).map_err(|e| format!("invalid username: it {e}"))?;
    let server = arguments
        .server_address
        .to_socket_addrs()
        .map_err(|e| {
            format!(
                "cannot use `{}` as a server address ({e}); expected host:port",
                arguments.server_address
            )
        })?
        .next()
        .ok_or("could not resolve the server address")?;

    let socket = UdpSocket::bind(if server.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })?;
    // A connected UDP socket only receives datagrams from the server.
    socket.connect(server)?;

    let (tx, rx) = mpsc::channel();
    spawn_socket_reader(socket.try_clone()?, tx.clone());
    spawn_stdin_reader(tx);

    let client_identifier = random_client_id();
    let mut client = Client::new(username, client_identifier);
    log_info!(
        "* connecting to {server} (client id {:#x})...",
        client_identifier
    );
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

        while let Some(event) = client.poll_event() {
            if let Some(code) = show(&event)? {
                exit_code = code;
            }
        }
        flush_transmits(&mut client, &socket)?;
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
        if let Some(deadline) = drain_deadline {
            wake = Some(wake.map_or(deadline, |w| w.min(deadline)));
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
fn flush_transmits(client: &mut Client, socket: &UdpSocket) -> io::Result<()> {
    while let Some(datagram) = client.poll_transmit() {
        //if let Ok(packet) = decode_packet(&datagram) {
        //log_error!("-> {packet:?}");
        log_debug!("-> {datagram:?}");
        //}
        match socket.send(&datagram) {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::ConnectionRefused | io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(e),
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
                    if tx.send(Input::Datagram(buf[..n].to_vec())).is_err() {
                        return;
                    }
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
                    if tx.send(Input::Line(line)).is_err() {
                        return;
                    }
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
