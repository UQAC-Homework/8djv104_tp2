//! Message identification and retransmission (RFC sections "Message
//! Identification" and "Message Retransmission").
//!
//! Each direction of the protocol is an independent stream of messages with
//! 8-bit identifiers that wrap around. A side therefore needs two pieces of
//! state:
//!
//! * [`ReceiveWindow`] – what *it* has received from the peer, reported back
//!   to the peer in every packet as `Last Message Identifier` + `Received
//!   Messages`.
//! * [`SendHistory`] – the last 64 messages *it* sent, so that it can
//!   retransmit the ones the peer reports as missing.

use crate::models::message::Message;
use crate::models::message_history::MessageHistory;

/// Size of the circular retransmission history.
pub const HISTORY_SIZE: usize = 64;

/// How an incoming message relates to the receive window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// Newer than anything seen. `distance` is `(id - last) % 256`; a value
    /// above 1 means messages were skipped and should be requested.
    New { distance: u8 },

    /// Older than `last` but not received before: fills a hole.
    Late,

    /// Already received, or too old to be tracked: must be ignored.
    Ignored,
}

impl Arrival {
    /// Checks if a message is missing
    pub fn is_message_missing(&self) -> bool {
        match self {
            Arrival::New { distance } => *distance > 1,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct Sent {
    id: u8,
    message: Message,
}

/// The last [`HISTORY_SIZE`] messages sent, indexed by `id % 64`.
#[derive(Debug, Clone)]
pub struct SendHistory {
    slots: Vec<Option<Sent>>,
    next_id: u8,
    sent: u64,
}

impl Default for SendHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SendHistory {
    pub fn new() -> Self {
        Self {
            slots: (0..HISTORY_SIZE).map(|_| None).collect(),
            next_id: 0,
            sent: 0,
        }
    }

    /// Stores `text` under the current counter value, advances the counter
    /// (wrapping) and returns the identifier to put in the packet. Must be
    /// called *before* the first emission.
    pub fn record(&mut self, message: Message) -> u8 {
        let id = self.next_id;
        self.slots[usize::from(id) % HISTORY_SIZE] = Some(Sent { id, message: message });
        self.next_id = id.wrapping_add(1);
        self.sent = self.sent.saturating_add(1);
        id
    }

    /// Identifier of the most recent message, if any was sent.
    pub fn last_sent(&self) -> Option<u8> {
        (self.sent > 0).then(|| self.next_id.wrapping_sub(1))
    }

    fn get(&self, id: u8) -> Option<&Message> {
        self.slots[usize::from(id) % HISTORY_SIZE]
            .as_ref()
            .filter(|s| s.id == id)
            .map(|s| &s.message)
    }

    /// True when the peer claims to have received a message newer than the
    /// last one sent. Such reports MUST be ignored.
    pub fn is_history_ahead(&self, history: MessageHistory) -> bool {
        match self.last_sent() {
            Some(last_sent) => history.is_head(last_sent),
            None => false,
        }
    }

    /// Messages that must be retransmitted according to the peer's `report`,
    /// oldest first.
    ///
    /// A message is missing if its bit is `0`, or if it is newer than the
    /// reported `last` and was already sent. Only the last 64 messages sent
    /// can be retransmitted; older ones are lost for good.
    pub fn missing(&self, history: MessageHistory) -> Vec<(u8, &Message)> {
        let Some(last_sent) = self.last_sent() else {
            return Vec::new();
        };

        if self.is_history_ahead(history) {
            return Vec::new();
        }

        let depth = self.sent.min(HISTORY_SIZE as u64) as u8;
        (0..depth)
            .rev() // oldest first
            .filter_map(|age| {
                let id = last_sent.wrapping_sub(age);

                if history.is_missing(id) {
                    self.get(id).map(|text| (id, text))
                } else {
                    None
                }
            })
            .collect()
    }
}
