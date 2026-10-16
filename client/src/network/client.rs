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
use crate::models::message_history::MessageHistory;
use crate::models::username::Username;
use crate::network::other::{Liveness, NotOnline, Session, State};
use crate::network::packet::MessageText;
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
use crate::network::reliability::{Arrival, SendHistory};
use crate::time::repeater::Repeater;
use crate::time::timeout::Timeout;
use std::collections::VecDeque;
use std::time::Instant;

pub struct Client {
    username: Username,
    client_id: u8,
    state: State,

    /// What we received from the server.
    server_history: MessageHistory,

    /// What we sent to the server.
    client_history: SendHistory,

    /// The latest usable report received from the server.
    latest_server_history: MessageHistory,
    outbox: VecDeque<Vec<u8>>,
    events: VecDeque<UserEvent>,
}

impl Client {
    /// Creates an offline client with a random `Client Identifier`.
    pub fn new(username: Username, client_identifier: u8) -> Self {
        Self {
            username,
            client_id: client_identifier & 0x0F,
            state: State::Offline,
            server_history: MessageHistory::EMPTY,
            client_history: SendHistory::new(),
            latest_server_history: MessageHistory::EMPTY,
            outbox: VecDeque::new(),
            events: VecDeque::new(),
        }
    }

    /// Checks if the client is currently online
    pub fn is_online(&self) -> bool {
        matches!(self.state, State::Online(_))
    }

    /// Checks if the client is currently offline
    pub fn is_offline(&self) -> bool {
        matches!(self.state, State::Offline)
    }

    /// True while some message we sent is not yet known to have arrived.
    pub fn has_unacknowledged(&self) -> bool {
        !self.client_history.missing(self.latest_server_history).is_empty()
    }

    // ---- user actions ------------------------------------------------------

    /// Offline -> Connecting: sends `CONNECTION_REQUEST`.
    pub fn connect(&mut self) {
        if !matches!(self.state, State::Offline) {
            panic!("Client cannot connect without being offline.")
        }

        // A new connection is a new session: both message streams restart.
        self.server_history = MessageHistory::EMPTY;
        self.client_history = SendHistory::new();
        self.latest_server_history = MessageHistory::EMPTY;
        self.state = State::Connecting {
            timeout_timer: Timeout::new(SERVER_CONNECTION_TIMEOUT),
            retry_repeater: Repeater::new(SERVER_CONNECTION_INTERVAL),
        };
        self.send_connection_request();
    }

    /// Sends and caches a `MESSAGE_SUBMIT`.
    pub fn send_message(&mut self, text: MessageText) -> Result<(), NotOnline> {
        let State::Online(session) = self.state else {
            return Err(NotOnline);
        };
        // The history is updated before the first emission.
        let message_id = self.client_history.record(text.clone());
        let packet = MessageSubmitPacket {
            user_identifier: session.user_id,
            message_history: self.server_history,
            message_identifier: message_id,
            message: text.to_string(),
        };
        self.transmit(&packet);
        Ok(())
    }

    /// Online -> Offline: sends `CONNECTION_CLOSED`. While connecting it
    /// simply abandons the attempt.
    pub fn disconnect(&mut self) {
        match self.state {
            State::Online(session) => {
                self.transmit(&ConnectionClosedPacket::close_session(&session));
                self.state = State::Offline;
                self.events.push_back(UserEvent::disconnect_by_user());
            }
            State::Connecting { .. } => self.state = State::Offline,
            State::Offline => {}
        }
    }

    // ---- network input -----------------------------------------------------

    /// Processes one received datagram. Malformed packets and unsupported
    /// types are discarded, as the RFC requires.
    pub fn handle_packet(&mut self, datagram: &[u8]) {
        let Ok(packet) = decode_packet(datagram) else {
            return;
        };
        match self.state {
            State::Offline => {}
            State::Connecting { .. } => self.on_connecting(packet),
            State::Online(_) => self.on_online(packet),
        }
    }

    fn on_connecting(&mut self, packet: NetworkPacket) {
        match packet {
            NetworkPacket::ConnectionAccepted(ConnectionAcceptedPacket { user_identifier }) => {
                self.state = State::Online(Session {
                    user_id: user_identifier,
                    liveness: Liveness::Idle {
                        inactivity_timeout: Timeout::new(INACTIVITY_WARNING),
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

    fn on_online(&mut self, packet: NetworkPacket) {
        match packet {
            // Only the reserved identifier 0 (server shutdown) ends the
            // session; any other value is ignored (RFC "Termination").
            NetworkPacket::ConnectionClosed(ConnectionClosedPacket { user_identifier }) => {
                if user_identifier != 0b000000 {
                    return;
                }

                self.state = State::Offline;
                self.events.push_back(UserEvent::disconnected_by_server());
            }
            NetworkPacket::MessageBroadcast(MessageBroadcastPacket {
                message_identifier,
                message_history,
                username,
                message,
            }) => {
                let arrival = self.server_history.mark_as_found(message_identifier);
                if let Arrival::New { distance } = arrival {
                    if distance > 1 {
                        self.request_missing();
                    }
                }
                // Duplicates carry fresh reports too.
                self.process_report(message_history);
                if arrival != Arrival::Ignored {
                    self.events
                        .push_back(UserEvent::new_message(username, message));
                }
            }
            NetworkPacket::MessageRequest(MessageRequestPacket {
                message_history, ..
            }) => self.process_report(message_history),
            NetworkPacket::HeartbeatReceived(HeartbeatReceivedPacket {
                message_history, ..
            }) => {
                self.process_report(message_history);
                self.back_to_idle();
            }
            NetworkPacket::HeartbeatRequest(HeartbeatRequestPacket {
                message_history, ..
            }) => {
                self.process_report(message_history);
                self.send_heartbeat();
                self.back_to_idle();
            }
            // Packets that only flow client -> server, repeats of
            // CONNECTION_ACCEPTED, CONNECTION_CLOSED for someone else, ...
            _ => {}
        }
    }

    /// Retransmits every message the server's report shows as missing.
    fn process_report(&mut self, history: MessageHistory) {
        if self.client_history.is_history_ahead(history) {
            return; // RFC: the fields MUST be ignored.
        }
        self.latest_server_history = history;
        let State::Online(session) = self.state else {
            return;
        };
        let resend: Vec<MessageSubmitPacket> = self
            .client_history
            .missing(history)
            .into_iter()
            .map(|(message_id, text)| MessageSubmitPacket {
                user_identifier: session.user_id,
                message_history: self.server_history,
                message_identifier: message_id,
                message: text.clone().to_string(),
            })
            .collect();
        for packet in &resend {
            self.transmit(packet);
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
                Liveness::Idle { inactivity_timeout } => {
                    Instant::now() + inactivity_timeout.remaining()
                }
                Liveness::Waiting {
                    heartbeat_interval: heartbeat_repeater,
                    server_timeout: server_timeout_timer,
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
    pub fn handle_timeout(&mut self) {
        match self.state {
            State::Offline => {}
            State::Connecting {
                timeout_timer,
                mut retry_repeater,
            } => {
                if timeout_timer.has_expired() {
                    self.state = State::Offline;
                    self.events.push_back(UserEvent::Timeout);
                } else if retry_repeater.pool() {
                    self.state = State::Connecting {
                        timeout_timer,
                        retry_repeater,
                    };
                    self.send_connection_request();
                }
            }
            State::Online(ref mut session) => match &mut session.liveness {
                Liveness::Idle { inactivity_timeout } => {
                    if inactivity_timeout.has_expired() {
                        session.liveness = Liveness::Waiting {
                            heartbeat_interval: Repeater::new(HEARTBEAT_INTERVAL),
                            server_timeout: Timeout::new(SERVER_INACTIVITY_TIMEOUT),
                        };
                        self.state = State::Online(*session);
                        self.send_heartbeat();
                    }
                }
                Liveness::Waiting {
                    server_timeout: server_timeout_timer,
                    heartbeat_interval: heartbeat_repeater,
                } => {
                    if server_timeout_timer.has_expired() {
                        let packet = ConnectionClosedPacket::close_session(session);
                        self.transmit(&packet);
                        self.state = State::Offline;
                        self.events.push_back(UserEvent::disconnected_by_timeout());
                    } else if heartbeat_repeater.pool() {
                        self.send_heartbeat();
                    }
                }
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
    fn transmit<P: Packet2>(&mut self, packet: &P) {
        self.outbox.push_back(encode_packet(packet));

        match &mut self.state {
            State::Online(Session {
                liveness: Liveness::Idle { inactivity_timeout },
                ..
            }) => {
                *inactivity_timeout = Timeout::new(INACTIVITY_WARNING);
            }
            _ => {}
        }
    }

    fn send_connection_request(&mut self) {
        let packet = ConnectionRequestPacket {
            client_identifier: self.client_id,
            username: self.username.clone(),
        };
        self.outbox.push_back(encode_packet(&packet));
    }

    fn send_heartbeat(&mut self) {
        if let Some(user_id) = self.user_id() {
            self.transmit(&HeartbeatBroadcastPacket {
                user_identifier: user_id,
                message_history: self.server_history,
            });
        }
    }

    fn request_missing(&mut self) {
        if let Some(user_id) = self.user_id() {
            self.transmit(&MessageRequestPacket {
                user_identifier: user_id,
                message_history: self.server_history,
            });
        }
    }

    /// Wait -> Online: the heartbeat timers stop and `INACTIVITY_WARNING`
    /// starts over.
    fn back_to_idle(&mut self) {
        match &mut self.state {
            State::Online(session) => {
                session.liveness = Liveness::Idle {
                    inactivity_timeout: Timeout::new(INACTIVITY_WARNING),
                };
            }
            _ => {}
        }
    }
}
