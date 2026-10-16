//! Command-line chat client.
//!
//! All protocol logic lives in the library; this file only moves bytes
//! between the UDP socket, the terminal and [`Client`], and sleeps until the
//! client's next timer.

use network::other::{DisconnectReason, Event};
use std::collections::VecDeque;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

mod network;
mod application;
mod time;
mod models;

use crate::application::arguments::Arguments;
use crate::models::username::Username;
use crate::network::client::Client;
use crate::network::packet::{MAX_MESSAGE_LEN, MessageText, Packet};

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
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = match Arguments::new(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprintln!("{USAGE}");
            return Ok(2);
        }
    };
    let username = Username::new(args.username).map_err(|e| format!("invalid username: it {e}"))?;
    let server = args
        .server_address
        .to_socket_addrs()
        .map_err(|e| {
            format!(
                "cannot use `{}` as a server address ({e}); expected host:port",
                args.server_address
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

    let verbose = args.verbose;
    let mut client = Client::new(username);
    eprintln!(
        "* connecting to {server} (client id {:#x})...",
        client.client_id()
    );
    client.connect();

    let mut exit_code = 0;
    let mut typed_before_online: VecDeque<MessageText> = VecDeque::new();
    let mut input_closed = false;
    let mut drain_deadline: Option<Instant> = None;

    loop {
        let now = Instant::now();

        // Lines typed while the connection was still being established.
        while client.is_online() {
            let Some(text) = typed_before_online.pop_front() else {
                break;
            };
            let _ = client.send_message(now, text);
        }

        while let Some(event) = client.poll_event() {
            if let Some(code) = show(&event)? {
                exit_code = code;
            }
        }
        flush_transmits(&mut client, &socket, verbose)?;
        if client.is_offline() {
            break;
        }

        // End of input: leave once the server has everything we sent.
        if input_closed && client.is_online() && typed_before_online.is_empty() {
            let deadline = *drain_deadline.get_or_insert(now + DRAIN_LIMIT);
            if !client.has_unacknowledged() || now >= deadline {
                client.disconnect(now);
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

        let now = Instant::now();
        match input {
            Ok(Input::Datagram(data)) => {
                if verbose {
                    match Packet::decode(&data) {
                        Ok(packet) => eprintln!("<- {packet:?}"),
                        Err(e) => eprintln!("<- discarded: {e}"),
                    }
                }
                client.handle_packet(now, &data);
            }
            Ok(Input::Line(line)) => {
                let line = line.trim_end_matches('\r');
                if line == "/quit" {
                    client.disconnect(now);
                } else if !line.is_empty() {
                    match MessageText::new(line) {
                        Ok(text) => typed_before_online.push_back(text),
                        Err(e) => {
                            eprintln!("! message not sent: it {e} (1-{MAX_MESSAGE_LEN} characters)")
                        }
                    }
                }
            }
            Ok(Input::StdinClosed) => input_closed = true,
            Ok(Input::SocketError(e)) => return Err(e.into()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        client.handle_timeout(now);
    }
    Ok(exit_code)
}

/// Sends everything the client queued. ICMP "connection refused" errors
/// surfacing on a connected UDP socket just mean the datagram was lost.
fn flush_transmits(client: &mut Client, socket: &UdpSocket, verbose: bool) -> io::Result<()> {
    while let Some(datagram) = client.poll_transmit() {
        if verbose {
            if let Ok(packet) = Packet::decode(&datagram) {
                eprintln!("-> {packet:?}");
            }
        }
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
fn show(event: &Event) -> io::Result<Option<i32>> {
    let mut out = io::stdout().lock();
    let code = match event {
        Event::Connected { user_id } => {
            eprintln!("* connected as user {user_id}");
            None
        }
        Event::Rejected => {
            eprintln!("* the server rejected the connection");
            Some(1)
        }
        Event::ConnectFailed => {
            eprintln!("* could not reach the server");
            Some(1)
        }
        Event::Message { sender, text, .. } => {
            // Other users control this text: keep control characters away
            // from the terminal.
            writeln!(out, "<{}> {}", printable(sender), printable(text))?;
            out.flush()?;
            None
        }
        Event::Disconnected(DisconnectReason::UserRequested) => {
            eprintln!("* disconnected");
            Some(0)
        }
        Event::Disconnected(DisconnectReason::ServerClosed) => {
            eprintln!("* the server closed the connection");
            Some(1)
        }
        Event::Disconnected(DisconnectReason::ServerUnreachable) => {
            eprintln!("* the server stopped answering; connection closed");
            Some(1)
        }
    };
    Ok(code)
}

fn printable(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_control() { '?' } else { c })
        .collect()
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
                    eprintln!("! line skipped: input is not valid text");
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(Input::StdinClosed);
    });
}
