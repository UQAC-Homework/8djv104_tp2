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

use crate::application::constants::{HEARTBEAT_INTERVAL, INACTIVITY_WARNING, SERVER_CONNECTION_INTERVAL, SERVER_CONNECTION_TIMEOUT, SERVER_INACTIVITY_TIMEOUT};
use crate::network::other::{DisconnectReason, Event, Liveness, NotOnline, Session, State};
use crate::network::packet::{MessageText, MissingMessages, Packet, Username};
use crate::network::reliability::{Arrival, ReceiveWindow, SendHistory};
use std::collections::VecDeque;
use std::time::Instant;

pub struct Client {
    username: Username,
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
    pub fn new(username: Username) -> Self {
        Self::with_client_id(username, random_client_id())
    }

    /// Like [`Client::new`] with a fixed identifier (only the low 4 bits are
    /// used). Mostly useful for tests.
    pub fn with_client_id(username: Username, client_id: u8) -> Self {
        Self {
            username,
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
            next_retry: now + SERVER_CONNECTION_INTERVAL,
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
                        warn_at: now + INACTIVITY_WARNING,
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
            } => Some(next_retry.min(started + SERVER_CONNECTION_TIMEOUT)),
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
        match self.state {
            State::Offline => {}
            State::Connecting {
                started,
                next_retry,
            } => {
                if now >= started + SERVER_CONNECTION_TIMEOUT {
                    self.state = State::Offline;
                    self.events.push_back(Event::ConnectFailed);
                } else if now >= next_retry {
                    self.state = State::Connecting {
                        started,
                        next_retry: now + SERVER_CONNECTION_INTERVAL,
                    };
                    self.send_connection_request();
                }
            }
            State::Online(mut session) => match session.liveness {
                Liveness::Idle { warn_at } if now >= warn_at => {
                    // INACTIVITY_WARNING expired: start heartbeating.
                    session.liveness = Liveness::Waiting {
                        next_heartbeat: now + HEARTBEAT_INTERVAL,
                        give_up_at: now + SERVER_INACTIVITY_TIMEOUT,
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
                            next_heartbeat: now + HEARTBEAT_INTERVAL,
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
        if let State::Online(Session {
            liveness: Liveness::Idle { warn_at },
            ..
        }) = &mut self.state
        {
            *warn_at = now + INACTIVITY_WARNING;
        }
    }

    fn send_connection_request(&mut self) {
        let packet = Packet::ConnectionRequest {
            client_id: self.client_id,
            username: self.username.clone(),
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
        if let State::Online(session) = &mut self.state {
            session.liveness = Liveness::Idle {
                warn_at: now + INACTIVITY_WARNING,
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
