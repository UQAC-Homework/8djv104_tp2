//! The client side of the protocol as a sans-IO state machine.
//!
//! [`Client`] never touches a socket or the clock. The caller feeds it
//! datagrams ([`Client::handle_packet`]), user actions ([`Client::connect`],
//! [`Client::send_message`], [`Client::disconnect`]) and timer expirations
//! ([`Client::handle_timeout`]), always passing the current [`Instant`]. It
//! answers through three queues: datagrams to send ([`Client::poll_transmit`]),
//! things to show the user ([`Client::poll_event`]) and the next time it needs
//! to be woken up ([`Client::poll_timeout`]).
//!
//! The RFC's client state machines map onto this module as follows:
//!
//! | RFC state machine | here                                               |
//! |-------------------|----------------------------------------------------|
//! | Connection        | `State::{Offline, Connecting, Online}`             |
//! | Heartbeat         | `Liveness::{Idle, Waiting}` inside `State::Online` |
//! | Data transfer     | `send_message` / `MessageBroadcast` handling       |
//! | Termination       | `disconnect`, `CONNECTION_CLOSED`, heartbeat give-up |

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use crate::network::packet::{MessageText, MissingMessages, Packet, Username};
use crate::network::reliability::{Arrival, ReceiveWindow, SendHistory};

/// The client-relevant protocol constants (RFC section "Constants").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timings {
    /// `HEARTBEAT_INTERVAL`: heartbeat period while waiting for the server.
    pub heartbeat_interval: Duration,
    /// `INACTIVITY_WARNING`: idle time before the client starts heartbeating.
    pub inactivity_warning: Duration,
    /// `SERVER_INACTIVITY_TIMEOUT`: silence after which the server is dead.
    pub server_inactivity_timeout: Duration,
    /// `SERVER_CONNECTION_INTERVAL`: retry period of `CONNECTION_REQUEST`.
    pub connection_interval: Duration,
    /// `SERVER_CONNECTION_TIMEOUT`: when to give up connecting.
    pub connection_timeout: Duration,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_millis(200),
            inactivity_warning: Duration::from_secs(1),
            server_inactivity_timeout: Duration::from_secs(3),
            connection_interval: Duration::from_secs(1),
            connection_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub username: Username,
    pub timings: Timings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The user asked for it.
    UserRequested,
    /// The server sent `CONNECTION_CLOSED` with the reserved identifier 0.
    ServerClosed,
    /// `SERVER_INACTIVITY_TIMEOUT` expired without an answer to heartbeats.
    ServerUnreachable,
}

/// Something the user interface should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Connected {
        user_id: u8,
    },
    /// The server answered `CONNECTION_REJECTED`.
    Rejected,
    /// `SERVER_CONNECTION_TIMEOUT` expired.
    ConnectFailed,
    /// A message to display; `late` is true when it filled a hole.
    Message {
        sender: String,
        text: String,
        late: bool,
    },
    Disconnected(DisconnectReason),
}

/// Returned by [`Client::send_message`] when there is no session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotOnline;

impl fmt::Display for NotOnline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not connected to a server")
    }
}

impl std::error::Error for NotOnline {}

#[derive(Debug, Clone, Copy)]
enum State {
    Offline,
    Connecting {
        started: Instant,
        next_retry: Instant,
    },
    Online(Session),
}

#[derive(Debug, Clone, Copy)]
struct Session {
    user_id: u8,
    liveness: Liveness,
}

/// The RFC's heartbeat state machine.
#[derive(Debug, Clone, Copy)]
enum Liveness {
    /// `Online`: `INACTIVITY_WARNING` is running and restarts whenever the
    /// client sends a packet.
    Idle { warn_at: Instant },
    /// `Wait`: heartbeats go out every `HEARTBEAT_INTERVAL` until the server
    /// answers or `SERVER_INACTIVITY_TIMEOUT` expires.
    Waiting {
        next_heartbeat: Instant,
        give_up_at: Instant,
    },
}

pub struct Client {
    config: Config,
    client_id: u8,
    state: State,
    /// What we received from the server.
    rx: ReceiveWindow,
    /// What we sent to the server.
    tx: SendHistory,
    /// The latest usable report received from the server.
    peer: MissingMessages,
    outbox: VecDeque<Vec<u8>>,
    events: VecDeque<Event>,
}

impl Client {
    /// Creates an offline client with a random `Client Identifier`.
    pub fn new(config: Config) -> Self {
        Self::with_client_id(config, random_client_id())
    }

    /// Like [`Client::new`] with a fixed identifier (only the low 4 bits are
    /// used). Mostly useful for tests.
    pub fn with_client_id(config: Config, client_id: u8) -> Self {
        Self {
            config,
            client_id: client_id & 0x0F,
            state: State::Offline,
            rx: ReceiveWindow::new(),
            tx: SendHistory::new(),
            peer: MissingMessages::INITIAL,
            outbox: VecDeque::new(),
            events: VecDeque::new(),
        }
    }

    pub fn client_id(&self) -> u8 {
        self.client_id
    }

    pub fn is_online(&self) -> bool {
        matches!(self.state, State::Online(_))
    }

    pub fn is_offline(&self) -> bool {
        matches!(self.state, State::Offline)
    }

    /// True while some message we sent is not yet known to have arrived.
    pub fn has_unacknowledged(&self) -> bool {
        !self.tx.missing(self.peer).is_empty()
    }

    // ---- user actions ------------------------------------------------------

    /// Offline -> Connecting: sends `CONNECTION_REQUEST`.
    pub fn connect(&mut self, now: Instant) {
        if !matches!(self.state, State::Offline) {
            return;
        }
        // A new connection is a new session: both message streams restart.
        self.rx = ReceiveWindow::new();
        self.tx = SendHistory::new();
        self.peer = MissingMessages::INITIAL;
        self.state = State::Connecting {
            started: now,
            next_retry: now + self.config.timings.connection_interval,
        };
        self.send_connection_request();
    }

    /// Sends and caches a `MESSAGE_SUBMIT`.
    pub fn send_message(&mut self, now: Instant, text: MessageText) -> Result<(), NotOnline> {
        let State::Online(session) = self.state else {
            return Err(NotOnline);
        };
        // The history is updated before the first emission.
        let message_id = self.tx.record(text.clone());
        let packet = Packet::MessageSubmit {
            user_id: session.user_id,
            report: self.rx.report(),
            message_id,
            message: text,
        };
        self.transmit(now, &packet);
        Ok(())
    }

    /// Online -> Offline: sends `CONNECTION_CLOSED`. While connecting it
    /// simply abandons the attempt.
    pub fn disconnect(&mut self, now: Instant) {
        match self.state {
            State::Online(session) => {
                self.transmit(
                    now,
                    &Packet::ConnectionClosed {
                        user_id: session.user_id,
                    },
                );
                self.state = State::Offline;
                self.events
                    .push_back(Event::Disconnected(DisconnectReason::UserRequested));
            }
            State::Connecting { .. } => self.state = State::Offline,
            State::Offline => {}
        }
    }

    // ---- network input -----------------------------------------------------

    /// Processes one received datagram. Malformed packets and unsupported
    /// types are discarded, as the RFC requires.
    pub fn handle_packet(&mut self, now: Instant, datagram: &[u8]) {
        let Ok(packet) = Packet::decode(datagram) else {
            return;
        };
        match self.state {
            State::Offline => {}
            State::Connecting { .. } => self.on_connecting(now, packet),
            State::Online(_) => self.on_online(now, packet),
        }
    }

    fn on_connecting(&mut self, now: Instant, packet: Packet) {
        match packet {
            Packet::ConnectionAccepted { user_id } => {
                self.state = State::Online(Session {
                    user_id,
                    liveness: Liveness::Idle {
                        warn_at: now + self.config.timings.inactivity_warning,
                    },
                });
                self.events.push_back(Event::Connected { user_id });
            }
            Packet::ConnectionRejected => {
                self.state = State::Offline;
                self.events.push_back(Event::Rejected);
            }
            _ => {}
        }
    }

    fn on_online(&mut self, now: Instant, packet: Packet) {
        match packet {
            // Only the reserved identifier 0 (server shutdown) ends the
            // session; any other value is ignored (RFC "Termination").
            Packet::ConnectionClosed { user_id: 0 } => {
                self.state = State::Offline;
                self.events
                    .push_back(Event::Disconnected(DisconnectReason::ServerClosed));
            }
            Packet::MessageBroadcast {
                message_id,
                report,
                username,
                message,
            } => {
                let arrival = self.rx.accept(message_id);
                if let Arrival::New { distance } = arrival {
                    if distance > 1 {
                        self.request_missing(now);
                    }
                }
                // Duplicates carry fresh reports too.
                self.process_report(now, report);
                if arrival != Arrival::Ignored {
                    self.events.push_back(Event::Message {
                        sender: username.to_string(),
                        text: message.to_string(),
                        late: arrival == Arrival::Late,
                    });
                }
            }
            Packet::MessageRequest { report, .. } => self.process_report(now, report),
            Packet::HeartbeatReceived { report } => {
                self.process_report(now, report);
                self.back_to_idle(now);
            }
            Packet::HeartbeatRequest { report } => {
                self.process_report(now, report);
                self.send_heartbeat(now);
                self.back_to_idle(now);
            }
            // Packets that only flow client -> server, repeats of
            // CONNECTION_ACCEPTED, CONNECTION_CLOSED for someone else, ...
            _ => {}
        }
    }

    /// Retransmits every message the server's report shows as missing.
    fn process_report(&mut self, now: Instant, report: MissingMessages) {
        if self.tx.report_is_ahead(report) {
            return; // RFC: the fields MUST be ignored.
        }
        self.peer = report;
        let State::Online(session) = self.state else {
            return;
        };
        let current = self.rx.report();
        let resend: Vec<Packet> = self
            .tx
            .missing(report)
            .into_iter()
            .map(|(message_id, text)| Packet::MessageSubmit {
                user_id: session.user_id,
                report: current,
                message_id,
                message: text.clone(),
            })
            .collect();
        for packet in &resend {
            self.transmit(now, packet);
        }
    }

    // ---- timers ------------------------------------------------------------

    /// The instant at which [`Client::handle_timeout`] needs to run next.
    pub fn poll_timeout(&self) -> Option<Instant> {
        match self.state {
            State::Offline => None,
            State::Connecting {
                started,
                next_retry,
            } => Some(next_retry.min(started + self.config.timings.connection_timeout)),
            State::Online(Session { liveness, .. }) => Some(match liveness {
                Liveness::Idle { warn_at } => warn_at,
                Liveness::Waiting {
                    next_heartbeat,
                    give_up_at,
                } => next_heartbeat.min(give_up_at),
            }),
        }
    }

    /// Runs whatever timer has expired at `now`.
    pub fn handle_timeout(&mut self, now: Instant) {
        let t = self.config.timings;
        match self.state {
            State::Offline => {}
            State::Connecting {
                started,
                next_retry,
            } => {
                if now >= started + t.connection_timeout {
                    self.state = State::Offline;
                    self.events.push_back(Event::ConnectFailed);
                } else if now >= next_retry {
                    self.state = State::Connecting {
                        started,
                        next_retry: now + t.connection_interval,
                    };
                    self.send_connection_request();
                }
            }
            State::Online(mut session) => match session.liveness {
                Liveness::Idle { warn_at } if now >= warn_at => {
                    // INACTIVITY_WARNING expired: start heartbeating.
                    session.liveness = Liveness::Waiting {
                        next_heartbeat: now + t.heartbeat_interval,
                        give_up_at: now + t.server_inactivity_timeout,
                    };
                    self.state = State::Online(session);
                    self.send_heartbeat(now);
                }
                Liveness::Waiting {
                    next_heartbeat,
                    give_up_at,
                } => {
                    if now >= give_up_at {
                        // SERVER_INACTIVITY_TIMEOUT expired.
                        self.transmit(
                            now,
                            &Packet::ConnectionClosed {
                                user_id: session.user_id,
                            },
                        );
                        self.state = State::Offline;
                        self.events
                            .push_back(Event::Disconnected(DisconnectReason::ServerUnreachable));
                    } else if now >= next_heartbeat {
                        session.liveness = Liveness::Waiting {
                            next_heartbeat: now + t.heartbeat_interval,
                            give_up_at,
                        };
                        self.state = State::Online(session);
                        self.send_heartbeat(now);
                    }
                }
                Liveness::Idle { .. } => {}
            },
        }
    }

    // ---- output queues -----------------------------------------------------

    /// Next datagram to send to the server.
    pub fn poll_transmit(&mut self) -> Option<Vec<u8>> {
        self.outbox.pop_front()
    }

    /// Next event to show to the user.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    // ---- helpers -----------------------------------------------------------

    fn user_id(&self) -> Option<u8> {
        match self.state {
            State::Online(session) => Some(session.user_id),
            _ => None,
        }
    }

    /// Queues a datagram. Sending restarts `INACTIVITY_WARNING`, but only
    /// while that timer is still running (i.e. not while waiting).
    fn transmit(&mut self, now: Instant, packet: &Packet) {
        self.outbox.push_back(packet.encode());
        let warning = self.config.timings.inactivity_warning;
        if let State::Online(Session {
            liveness: Liveness::Idle { warn_at },
            ..
        }) = &mut self.state
        {
            *warn_at = now + warning;
        }
    }

    fn send_connection_request(&mut self) {
        let packet = Packet::ConnectionRequest {
            client_id: self.client_id,
            username: self.config.username.clone(),
        };
        self.outbox.push_back(packet.encode());
    }

    fn send_heartbeat(&mut self, now: Instant) {
        if let Some(user_id) = self.user_id() {
            let report = self.rx.report();
            self.transmit(now, &Packet::HeartbeatBroadcast { user_id, report });
        }
    }

    fn request_missing(&mut self, now: Instant) {
        if let Some(user_id) = self.user_id() {
            let report = self.rx.report();
            self.transmit(now, &Packet::MessageRequest { user_id, report });
        }
    }

    /// Wait -> Online: the heartbeat timers stop and `INACTIVITY_WARNING`
    /// starts over.
    fn back_to_idle(&mut self, now: Instant) {
        let warning = self.config.timings.inactivity_warning;
        if let State::Online(session) = &mut self.state {
            session.liveness = Liveness::Idle {
                warn_at: now + warning,
            };
        }
    }
}

fn random_client_id() -> u8 {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    // `RandomState` is seeded randomly per process; four bits are enough.
    (RandomState::new().hash_one(std::process::id()) & 0x0F) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE_ID: u8 = 5;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn new_client() -> Client {
        Client::with_client_id(
            Config {
                username: Username::new("alice").unwrap(),
                timings: Timings::default(),
            },
            3,
        )
    }

    fn sent(c: &mut Client) -> Vec<Packet> {
        std::iter::from_fn(|| c.poll_transmit())
            .map(|bytes| Packet::decode(&bytes).expect("client sent a valid packet"))
            .collect()
    }

    fn events(c: &mut Client) -> Vec<Event> {
        std::iter::from_fn(|| c.poll_event()).collect()
    }

    fn feed(c: &mut Client, now: Instant, packet: Packet) {
        c.handle_packet(now, &packet.encode());
    }

    /// A client that is online (user id 5), with its queues drained.
    fn online(t0: Instant) -> Client {
        let mut c = new_client();
        c.connect(t0);
        feed(&mut c, t0, Packet::ConnectionAccepted { user_id: ALICE_ID });
        sent(&mut c);
        assert_eq!(events(&mut c), vec![Event::Connected { user_id: ALICE_ID }]);
        c
    }

    fn broadcast(id: u8, last: u8, text: &str) -> Packet {
        Packet::MessageBroadcast {
            message_id: id,
            report: MissingMessages {
                last,
                received: u64::MAX,
            },
            username: Username::new("bob").unwrap(),
            message: MessageText::new(text).unwrap(),
        }
    }

    fn submits(packets: &[Packet]) -> Vec<(u8, String)> {
        packets
            .iter()
            .filter_map(|p| match p {
                Packet::MessageSubmit {
                    message_id,
                    message,
                    ..
                } => Some((*message_id, message.to_string())),
                _ => None,
            })
            .collect()
    }

    // ---- connection --------------------------------------------------------

    #[test]
    fn connect_sends_request_with_client_id_and_username() {
        let t0 = Instant::now();
        let mut c = new_client();
        c.connect(t0);
        assert_eq!(
            sent(&mut c),
            vec![Packet::ConnectionRequest {
                client_id: 3,
                username: Username::new("alice").unwrap()
            }]
        );
    }

    #[test]
    fn connection_request_is_retried_then_abandoned() {
        let t0 = Instant::now();
        let mut c = new_client();
        c.connect(t0);
        sent(&mut c);

        c.handle_timeout(t0 + ms(500));
        assert!(sent(&mut c).is_empty());
        assert_eq!(c.poll_timeout(), Some(t0 + ms(1000)));

        for s in 1..=9 {
            c.handle_timeout(t0 + ms(1000 * s));
            let out = sent(&mut c);
            assert_eq!(out.len(), 1, "retry at {s}s");
            assert!(matches!(
                out[0],
                Packet::ConnectionRequest { client_id: 3, .. }
            ));
        }
        c.handle_timeout(t0 + ms(10_000));
        assert!(sent(&mut c).is_empty());
        assert_eq!(events(&mut c), vec![Event::ConnectFailed]);
        assert!(c.is_offline());
    }

    #[test]
    fn rejected_connection_goes_offline() {
        let t0 = Instant::now();
        let mut c = new_client();
        c.connect(t0);
        feed(&mut c, t0, Packet::ConnectionRejected);
        assert_eq!(events(&mut c), vec![Event::Rejected]);
        assert!(c.is_offline());
    }

    #[test]
    fn accepted_connection_goes_online() {
        let t0 = Instant::now();
        let c = online(t0);
        assert!(c.is_online());
        assert_eq!(c.poll_timeout(), Some(t0 + ms(1000)));
    }

    // ---- heartbeat ---------------------------------------------------------

    #[test]
    fn idle_client_heartbeats_and_returns_to_idle_on_answer() {
        let t0 = Instant::now();
        let mut c = online(t0);

        c.handle_timeout(t0 + ms(999));
        assert!(sent(&mut c).is_empty());

        c.handle_timeout(t0 + ms(1000));
        assert_eq!(
            sent(&mut c),
            vec![Packet::HeartbeatBroadcast {
                user_id: ALICE_ID,
                report: MissingMessages::INITIAL
            }]
        );
        assert_eq!(c.poll_timeout(), Some(t0 + ms(1200)));

        feed(
            &mut c,
            t0 + ms(1050),
            Packet::HeartbeatReceived {
                report: MissingMessages::INITIAL,
            },
        );
        assert!(sent(&mut c).is_empty());
        // Back to Online: INACTIVITY_WARNING restarted, no more heartbeats
        // until it expires again.
        assert_eq!(c.poll_timeout(), Some(t0 + ms(2050)));
        c.handle_timeout(t0 + ms(1500));
        assert!(sent(&mut c).is_empty());
    }

    #[test]
    fn unanswered_heartbeats_repeat_then_connection_is_closed() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.handle_timeout(t0 + ms(1000));
        sent(&mut c);

        // Heartbeats every 200 ms during the 3 s the server has to answer.
        let mut heartbeats = 0;
        for step in 1..15 {
            c.handle_timeout(t0 + ms(1000 + 200 * step));
            heartbeats += sent(&mut c)
                .iter()
                .filter(|p| matches!(p, Packet::HeartbeatBroadcast { .. }))
                .count();
        }
        assert_eq!(heartbeats, 14);
        assert!(c.is_online());

        c.handle_timeout(t0 + ms(4000));
        assert_eq!(
            sent(&mut c),
            vec![Packet::ConnectionClosed { user_id: ALICE_ID }]
        );
        assert_eq!(
            events(&mut c),
            vec![Event::Disconnected(DisconnectReason::ServerUnreachable)]
        );
        assert!(c.is_offline());
    }

    #[test]
    fn heartbeat_request_is_answered() {
        let t0 = Instant::now();
        let mut c = online(t0);
        feed(
            &mut c,
            t0 + ms(300),
            Packet::HeartbeatRequest {
                report: MissingMessages::INITIAL,
            },
        );
        assert_eq!(
            sent(&mut c),
            vec![Packet::HeartbeatBroadcast {
                user_id: ALICE_ID,
                report: MissingMessages::INITIAL
            }]
        );
        // The send restarted the idle timer.
        assert_eq!(c.poll_timeout(), Some(t0 + ms(1300)));
    }

    #[test]
    fn sending_restarts_the_idle_timer_but_not_the_wait() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.send_message(t0 + ms(800), MessageText::new("hi").unwrap())
            .unwrap();
        assert_eq!(c.poll_timeout(), Some(t0 + ms(1800)));
        sent(&mut c);

        c.handle_timeout(t0 + ms(1800));
        sent(&mut c);
        let before = c.poll_timeout();
        c.send_message(t0 + ms(1850), MessageText::new("again").unwrap())
            .unwrap();
        // Waiting: the heartbeat schedule is untouched by sending.
        assert_eq!(c.poll_timeout(), before);
    }

    // ---- data transfer -----------------------------------------------------

    #[test]
    fn submitted_messages_carry_user_id_counter_and_report() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.send_message(t0, MessageText::new("Hello World!").unwrap())
            .unwrap();
        c.send_message(t0, MessageText::new("second").unwrap())
            .unwrap();
        assert_eq!(
            sent(&mut c),
            vec![
                Packet::MessageSubmit {
                    user_id: ALICE_ID,
                    report: MissingMessages::INITIAL,
                    message_id: 0,
                    message: MessageText::new("Hello World!").unwrap()
                },
                Packet::MessageSubmit {
                    user_id: ALICE_ID,
                    report: MissingMessages::INITIAL,
                    message_id: 1,
                    message: MessageText::new("second").unwrap()
                },
            ]
        );
    }

    #[test]
    fn cannot_send_while_offline() {
        let mut c = new_client();
        assert_eq!(
            c.send_message(Instant::now(), MessageText::new("x").unwrap()),
            Err(NotOnline)
        );
    }

    #[test]
    fn broadcasts_are_displayed_immediately_and_in_arrival_order() {
        let t0 = Instant::now();
        let mut c = online(t0);
        feed(&mut c, t0, broadcast(0, 0xFF, "one"));
        feed(&mut c, t0, broadcast(1, 0xFF, "two"));
        assert_eq!(
            events(&mut c),
            vec![
                Event::Message {
                    sender: "bob".into(),
                    text: "one".into(),
                    late: false
                },
                Event::Message {
                    sender: "bob".into(),
                    text: "two".into(),
                    late: false
                },
            ]
        );
        assert!(sent(&mut c).is_empty());
    }

    #[test]
    fn rfc_example_hole_is_requested_and_filled_late() {
        let t0 = Instant::now();
        let mut c = online(t0);
        for id in 0..=4 {
            feed(&mut c, t0, broadcast(id, 0xFF, "m"));
        }
        events(&mut c);
        sent(&mut c);

        // Message 5 is lost; 6 reveals the hole.
        feed(&mut c, t0, broadcast(6, 0xFF, "six"));
        assert_eq!(events(&mut c).len(), 1, "6 is displayed without waiting");
        let out = sent(&mut c);
        assert_eq!(out.len(), 1);
        let Packet::MessageRequest { user_id, report } = &out[0] else {
            panic!("expected MESSAGE_REQUEST, got {:?}", out[0]);
        };
        assert_eq!(*user_id, ALICE_ID);
        assert_eq!(report.last, 6);
        assert_eq!(report.received & 1, 0, "bit 0 = message 5 is missing");
        assert_eq!(report.received & 2, 2, "bit 1 = message 4 is present");

        // The retransmission arrives and is displayed as late.
        feed(&mut c, t0, broadcast(5, 0xFF, "five"));
        assert_eq!(
            events(&mut c),
            vec![Event::Message {
                sender: "bob".into(),
                text: "five".into(),
                late: true
            }]
        );
        // A duplicate is not displayed again.
        feed(&mut c, t0, broadcast(5, 0xFF, "five"));
        assert!(events(&mut c).is_empty());
        assert!(sent(&mut c).is_empty());
    }

    #[test]
    fn rfc_example_lost_last_message_is_recovered_by_heartbeat() {
        let t0 = Instant::now();
        let mut c = online(t0);
        // Server has everything up to message 6; the client sends 0..=7 and
        // message 7 is lost.
        for i in 0..=7 {
            c.send_message(t0, MessageText::new(format!("m{i}")).unwrap())
                .unwrap();
        }
        sent(&mut c);

        c.handle_timeout(t0 + ms(1000));
        assert!(matches!(sent(&mut c)[0], Packet::HeartbeatBroadcast { .. }));

        feed(
            &mut c,
            t0 + ms(1020),
            Packet::HeartbeatReceived {
                report: MissingMessages {
                    last: 6,
                    received: u64::MAX,
                },
            },
        );
        assert_eq!(submits(&sent(&mut c)), vec![(7, "m7".to_string())]);
        assert!(c.has_unacknowledged());

        // The server finally acknowledges everything.
        feed(&mut c, t0 + ms(1040), broadcast(0, 7, "echo"));
        assert!(submits(&sent(&mut c)).is_empty());
        assert!(!c.has_unacknowledged());
    }

    #[test]
    fn message_request_triggers_retransmission_with_original_identifier() {
        let t0 = Instant::now();
        let mut c = online(t0);
        for i in 0..4 {
            c.send_message(t0, MessageText::new(format!("m{i}")).unwrap())
                .unwrap();
        }
        sent(&mut c);

        // Server saw 3 but not 1 (bit 3 - 1 - 1 = 1).
        feed(
            &mut c,
            t0,
            Packet::MessageRequest {
                user_id: ALICE_ID,
                report: MissingMessages {
                    last: 3,
                    received: !(1u64 << 1),
                },
            },
        );
        assert_eq!(submits(&sent(&mut c)), vec![(1, "m1".to_string())]);
    }

    #[test]
    fn retransmissions_carry_the_current_report() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.send_message(t0, MessageText::new("m0").unwrap()).unwrap();
        sent(&mut c);
        feed(&mut c, t0, broadcast(0, 0xFF, "hello")); // acks nothing, we got 0
        let out = sent(&mut c);
        let Packet::MessageSubmit { report, .. } = &out[0] else {
            panic!("expected a retransmission");
        };
        assert_eq!(report.last, 0);
    }

    #[test]
    fn reports_newer_than_anything_sent_are_ignored() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.send_message(t0, MessageText::new("m0").unwrap()).unwrap();
        sent(&mut c);
        feed(
            &mut c,
            t0,
            Packet::HeartbeatReceived {
                report: MissingMessages {
                    last: 50,
                    received: 0,
                },
            },
        );
        assert!(sent(&mut c).is_empty());
    }

    // ---- termination -------------------------------------------------------

    #[test]
    fn user_disconnect_sends_connection_closed() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.disconnect(t0);
        assert_eq!(
            sent(&mut c),
            vec![Packet::ConnectionClosed { user_id: ALICE_ID }]
        );
        assert_eq!(
            events(&mut c),
            vec![Event::Disconnected(DisconnectReason::UserRequested)]
        );
        assert!(c.is_offline());
        assert_eq!(c.poll_timeout(), None);
    }

    #[test]
    fn server_shutdown_uses_identifier_zero() {
        let t0 = Instant::now();
        let mut c = online(t0);
        feed(&mut c, t0, Packet::ConnectionClosed { user_id: 9 });
        assert!(c.is_online(), "closed for someone else: ignored");
        feed(&mut c, t0, Packet::ConnectionClosed { user_id: 0 });
        assert_eq!(
            events(&mut c),
            vec![Event::Disconnected(DisconnectReason::ServerClosed)]
        );
        assert!(c.is_offline());
    }

    #[test]
    fn garbage_and_unknown_packets_are_discarded() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.handle_packet(t0, &[]);
        c.handle_packet(t0, &[0x00]);
        c.handle_packet(t0, &[0x99, 1, 2, 3]);
        c.handle_packet(t0, &[0x11, 0x00]);
        assert!(c.is_online());
        assert!(events(&mut c).is_empty());
        assert!(sent(&mut c).is_empty());
    }

    #[test]
    fn reconnecting_starts_fresh_streams() {
        let t0 = Instant::now();
        let mut c = online(t0);
        c.send_message(t0, MessageText::new("old").unwrap())
            .unwrap();
        feed(&mut c, t0, broadcast(0, 0, "x"));
        c.disconnect(t0);
        sent(&mut c);
        events(&mut c);

        c.connect(t0 + ms(10));
        sent(&mut c);
        feed(
            &mut c,
            t0 + ms(20),
            Packet::ConnectionAccepted { user_id: 8 },
        );
        c.send_message(t0 + ms(30), MessageText::new("new").unwrap())
            .unwrap();
        let out = sent(&mut c);
        assert_eq!(
            out,
            vec![Packet::MessageSubmit {
                user_id: 8,
                report: MissingMessages::INITIAL,
                message_id: 0,
                message: MessageText::new("new").unwrap()
            }]
        );
    }
}
