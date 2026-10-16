use crate::network::packets::packet::{Packet, encode_packet};
use std::collections::VecDeque;

/// Structure used to queue [Packet]
pub struct PacketQueue {
    queue: VecDeque<Vec<u8>>,
}

impl PacketQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Adds the given packet to the queue
    pub fn push<P: Packet>(&mut self, packet: &P) {
        let data = encode_packet(packet);
        self.queue.push_back(data);
    }

    /// Attempts to remove a packet from the queue
    pub fn pull(&mut self) -> Option<Vec<u8>> {
        self.queue.pop_front()
    }
}
