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

use crate::application::constants::{
    HEARTBEAT_INTERVAL, INACTIVITY_WARNING, SERVER_CONNECTION_INTERVAL, SERVER_CONNECTION_TIMEOUT,
    SERVER_INACTIVITY_TIMEOUT,
};
use crate::application::events::event::UserEvent;
use crate::models::username::Username;
use crate::network::other::{DisconnectReason, Liveness, NotOnline, Session, State};
use crate::network::packet::{MessageText, MissingMessages};
use crate::network::packets::connection_accepted::ConnectionAcceptedPacket;
use crate::network::packets::connection_closed::ConnectionClosedPacket;
use crate::network::packets::connection_request::ConnectionRequestPacket;
use crate::network::packets::heartbeat_broadcast::HeartbeatBroadcastPacket;
use crate::network::packets::heartbeat_received::HeartbeatReceivedPacket;
use crate::network::packets::heartbeat_request::HeartbeatRequestPacket;
use crate::network::packets::message_broadcast::MessageBroadcastPacket;
use crate::network::packets::message_request::MessageRequestPacket;
use crate::network::packets::message_submit::MessageSubmitPacket;
use crate::network::packets::packet::{
    NetworkPacket, Packet as Packet2, decode_packet, encode_packet,
};
use crate::network::reliability::{Arrival, ReceiveWindow, SendHistory};
use crate::time::repeater::Repeater;
use crate::time::timeout::Timeout;
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
    events: VecDeque<UserEvent>,
}

impl Client {
    /// Creates an offline client with a random `Client Identifier`.
    pub fn new(username: Username) -> Self {
        let client_identifier = random_client_id();

        Self {
            username,
            client_id: client_identifier & 0x0F,
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
    pub fn connect(&mut self) {
        if !matches!(self.state, State::Offline) {
            return;
        }
        // A new connection is a new session: both message streams restart.
        self.rx = ReceiveWindow::new();
        self.tx = SendHistory::new();
        self.peer = MissingMessages::INITIAL;
        self.state = State::Connecting {
            timeout_timer: Timeout::new(SERVER_CONNECTION_TIMEOUT),
            retry_repeater: Repeater::new(SERVER_CONNECTION_INTERVAL),
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
        let packet = MessageSubmitPacket {
            user_identifier: session.user_id,
            last_message_identifier: self.rx.report().last,
            received_messages: self.rx.report().received,
            message_identifier: message_id,
            message: text.to_string(),
        };
        self.transmit(now, &packet);
        Ok(())
    }

    /// Online -> Offline: sends `CONNECTION_CLOSED`. While connecting it
    /// simply abandons the attempt.
    pub fn disconnect(&mut self, now: Instant) {
        match self.state {
            State::Online(session) => {
                self.transmit(now, &ConnectionClosedPacket::close_session(session));
                self.state = State::Offline;
                self.events
                    .push_back(UserEvent::disconnect_by_user());
            }
            State::Connecting { .. } => self.state = State::Offline,
            State::Offline => {}
        }
    }

    // ---- network input -----------------------------------------------------

    /// Processes one received datagram. Malformed packets and unsupported
    /// types are discarded, as the RFC requires.
    pub fn handle_packet(&mut self, now: Instant, datagram: &[u8]) {
        let Ok(packet) = decode_packet(datagram) else {
            return;
        };
        match self.state {
            State::Offline => {}
            State::Connecting { .. } => self.on_connecting(now, packet),
            State::Online(_) => self.on_online(now, packet),
        }
    }

    fn on_connecting(&mut self, now: Instant, packet: NetworkPacket) {
        match packet {
            NetworkPacket::ConnectionAccepted(ConnectionAcceptedPacket { user_identifier }) => {
                self.state = State::Online(Session {
                    user_id: user_identifier,
                    liveness: Liveness::Idle {
                        warn_at: now + INACTIVITY_WARNING,
                    },
                });
                self.events.push_back(UserEvent::Connected {});
            }
            NetworkPacket::ConnectionRejected(_) => {
                self.state = State::Offline;
                self.events.push_back(UserEvent::Rejected);
            }
            _ => {}
        }
    }

    fn on_online(&mut self, now: Instant, packet: NetworkPacket) {
        match packet {
            // Only the reserved identifier 0 (server shutdown) ends the
            // session; any other value is ignored (RFC "Termination").
            NetworkPacket::ConnectionClosed(ConnectionClosedPacket { user_identifier }) => {
                if user_identifier != 0b000000 {
                    return;
                }

                self.state = State::Offline;
                self.events
                    .push_back(UserEvent::disconnected_by_server());
            }
            NetworkPacket::MessageBroadcast(MessageBroadcastPacket {
                message_identifier,
                last_message_identifier,
                received_messages,
                username,
                message,
            }) => {
                let report = MissingMessages {
                    last: last_message_identifier,
                    received: received_messages,
                };
                let arrival = self.rx.accept(message_identifier);
                if let Arrival::New { distance } = arrival {
                    if distance > 1 {
                        self.request_missing(now);
                    }
                }
                // Duplicates carry fresh reports too.
                self.process_report(now, report);
                if arrival != Arrival::Ignored {
                    self.events.push_back(UserEvent::new_message(username, message));
                }
            }
            NetworkPacket::MessageRequest(MessageRequestPacket {
                last_message_identifier,
                received_messages,
                ..
            }) => {
                let report = MissingMessages {
                    last: last_message_identifier,
                    received: received_messages,
                };

                self.process_report(now, report)
            }
            NetworkPacket::HeartbeatReceived(HeartbeatReceivedPacket {
                last_message_identifier,
                received_messages,
                ..
            }) => {
                let report = MissingMessages {
                    last: last_message_identifier,
                    received: received_messages,
                };

                self.process_report(now, report);
                self.back_to_idle(now);
            }
            NetworkPacket::HeartbeatRequest(HeartbeatRequestPacket {
                last_message_identifier,
                received_messages,
                ..
            }) => {
                let report = MissingMessages {
                    last: last_message_identifier,
                    received: received_messages,
                };
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
        let resend: Vec<MessageSubmitPacket> = self
            .tx
            .missing(report)
            .into_iter()
            .map(|(message_id, text)| MessageSubmitPacket {
                user_identifier: session.user_id,
                last_message_identifier: current.last,
                received_messages: current.received,
                message_identifier: message_id,
                message: text.clone().to_string(),
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
                timeout_timer,
                retry_repeater,
            } => {
                let minimum_remaining = retry_repeater.remaining().min(timeout_timer.remaining());

                Some(Instant::now() + minimum_remaining)
            }
            State::Online(Session { liveness, .. }) => Some(match liveness {
                Liveness::Idle { warn_at } => warn_at,
                Liveness::Waiting {
                    heartbeat_repeater,
                    server_timeout_timer,
                } => {
                    let minimum_remaining = heartbeat_repeater
                        .remaining()
                        .min(server_timeout_timer.remaining());

                    Instant::now() + minimum_remaining
                }
            }),
        }
    }

    /// Runs whatever timer has expired at `now`.
    pub fn handle_timeout(&mut self, now: Instant) {
        match self.state {
            State::Offline => {}
            State::Connecting {
                timeout_timer,
                mut retry_repeater,
            } => {
                if timeout_timer.has_expired() {
                    self.state = State::Offline;
                    self.events.push_back(UserEvent::Timeout);
                } else if retry_repeater.is_due() {
                    retry_repeater.reset();
                    self.state = State::Connecting {
                        timeout_timer,
                        retry_repeater,
                    };
                    self.send_connection_request();
                }
            }
            State::Online(mut session) => match session.liveness {
                Liveness::Idle { warn_at } if now >= warn_at => {
                    // INACTIVITY_WARNING expired: start heartbeating.
                    session.liveness = Liveness::Waiting {
                        heartbeat_repeater: Repeater::new(HEARTBEAT_INTERVAL),
                        server_timeout_timer: Timeout::new(SERVER_INACTIVITY_TIMEOUT),
                    };
                    self.state = State::Online(session);
                    self.send_heartbeat(now);
                }
                Liveness::Waiting {
                    server_timeout_timer,
                    mut heartbeat_repeater,
                } => {
                    if server_timeout_timer.has_expired() {
                        self.transmit(now, &ConnectionClosedPacket::close_session(session));
                        self.state = State::Offline;
                        self.events
                            .push_back(UserEvent::disconnected_by_timeout());
                    } else if heartbeat_repeater.is_due() {
                        heartbeat_repeater.reset();

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
    pub fn poll_event(&mut self) -> Option<UserEvent> {
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
    fn transmit<P: Packet2>(&mut self, now: Instant, packet: &P) {
        self.outbox.push_back(encode_packet(packet));

        if let State::Online(Session {
            liveness: Liveness::Idle { warn_at },
            ..
        }) = &mut self.state
        {
            *warn_at = now + INACTIVITY_WARNING;
        }
    }

    fn send_connection_request(&mut self) {
        let packet = ConnectionRequestPacket {
            client_identifier: self.client_id,
            username: self.username.clone(),
        };
        self.outbox.push_back(encode_packet(&packet));
    }

    fn send_heartbeat(&mut self, now: Instant) {
        if let Some(user_id) = self.user_id() {
            let report = self.rx.report();
            self.transmit(
                now,
                &HeartbeatBroadcastPacket {
                    user_identifier: user_id,
                    last_message_identifier: report.last,
                    received_messages: report.received,
                },
            );
        }
    }

    fn request_missing(&mut self, now: Instant) {
        if let Some(user_id) = self.user_id() {
            let report = self.rx.report();
            self.transmit(
                now,
                &MessageRequestPacket {
                    user_identifier: user_id,
                    last_message_identifier: report.last,
                    received_messages: report.received,
                },
            );
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
