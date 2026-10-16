use crate::network::other::DisconnectReason;
use std::fmt;

/// List of possible events to show to the user
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEvent {
    /// Called when the local user connects to the server
    Connected,

    /// Called when the server rejects the connection request
    Rejected,

    /// Called when the connection request times out
    Timeout,

    /// Called when a message is broadcasted for the first time
    Message { sender: String, text: String },

    /// Called when the client gets disconnected
    Disconnected(DisconnectReason),
}

impl fmt::Display for UserEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            UserEvent::Connected => "You are connected.",
            UserEvent::Rejected => "Server has rejected your connection.",
            UserEvent::Timeout => "Could not reach the server.",
            UserEvent::Message { sender, text } => &*format!("<{}> {}", sender, text),
            UserEvent::Disconnected(DisconnectReason::UserRequested) => "You are disconnected.",
            UserEvent::Disconnected(DisconnectReason::ServerClosed) => {
                "Server has closed your connection."
            }
            UserEvent::Disconnected(DisconnectReason::ServerUnreachable) => {
                "Could not reach server further."
            }
        };

        code.fmt(f)
    }
}

impl UserEvent {
    /// Creates a new event for the new given message
    pub fn new_message(username: String, text: String) -> UserEvent {
        UserEvent::Message {
            sender: username,
            text,
        }
    }

    /// Creates a new event for when the client disconnects by the user
    pub fn disconnect_by_user() -> UserEvent {
        UserEvent::Disconnected(DisconnectReason::UserRequested)
    }
    
    /// Creates a new event for when the client disconnects by the server
    pub fn disconnected_by_server() -> UserEvent {
        UserEvent::Disconnected(DisconnectReason::ServerClosed)
    }
    
    /// Creates a new event for when the client disconnects by a timeout
    pub fn disconnected_by_timeout() -> UserEvent {
        UserEvent::Disconnected(DisconnectReason::ServerUnreachable)
    }
}
