use crate::time::repeater::Repeater;
use crate::time::timeout::Timeout;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The user asked for it.
    UserRequested,
    /// The server sent `CONNECTION_CLOSED` with the reserved identifier 0.
    ServerClosed,
    /// `SERVER_INACTIVITY_TIMEOUT` expired without an answer to heartbeats.
    ServerUnreachable,
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

#[derive(Debug, Clone)]
pub enum State {
    Offline,
    Connecting {
        timeout_timer: Timeout,
        retry_repeater: Repeater,
    },
    Online(Session),
}

#[derive(Debug, Clone, Copy)]
pub struct Session {
    pub user_id: u8,
    pub liveness: Liveness,
}

/// The RFC's heartbeat state machine.
#[derive(Debug, Clone, Copy)]
pub enum Liveness {
    /// `Online`: `INACTIVITY_WARNING` is running and restarts whenever the
    /// client sends a packet.
    Idle {
        inactivity_timeout: Timeout,
    },
    /// `Wait`: heartbeats go out every `HEARTBEAT_INTERVAL` until the server
    /// answers or `SERVER_INACTIVITY_TIMEOUT` expires.
    Waiting {
        server_timeout: Timeout,
        heartbeat_interval: Repeater,
    },
}
