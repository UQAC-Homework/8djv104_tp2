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

use crate::network::packet::{MessageText, MissingMessages};

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

/// Tracks which messages of the peer's stream have been received.
#[derive(Debug, Clone)]
pub struct ReceiveWindow {
    report: MissingMessages,
}

impl Default for ReceiveWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiveWindow {
    pub fn new() -> Self {
        Self {
            report: MissingMessages::INITIAL,
        }
    }

    /// The `Last Message Identifier` / `Received Messages` to put in
    /// outgoing packets.
    pub fn report(&self) -> MissingMessages {
        self.report
    }

    /// Classifies message `id` and updates the window accordingly.
    pub fn accept(&mut self, id: u8) -> Arrival {
        let ahead = id.wrapping_sub(self.report.last);
        if (1..=127).contains(&ahead) {
            // New message. Bit n describes `last - 1 - n`, so moving `last`
            // forward by `ahead` shifts the old bits up by `ahead`, marks
            // the old `last` as received (bit `ahead - 1`) and leaves the
            // identifiers in between as missing (0).
            let shifted = if ahead >= 64 {
                0
            } else {
                self.report.received << ahead
            };
            let old_last = if ahead <= 64 { 1u64 << (ahead - 1) } else { 0 };
            self.report = MissingMessages {
                last: id,
                received: shifted | old_last,
            };
            return Arrival::New { distance: ahead };
        }

        match self.report.last.wrapping_sub(id) {
            1..=64 => {
                let bit = 1u64 << (self.report.last.wrapping_sub(id) - 1);
                if self.report.received & bit != 0 {
                    Arrival::Ignored
                } else {
                    self.report.received |= bit;
                    Arrival::Late
                }
            }
            // 0: duplicate of `last`; above 64: older than the window.
            _ => Arrival::Ignored,
        }
    }
}

#[derive(Debug, Clone)]
struct Sent {
    id: u8,
    text: MessageText,
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
    pub fn record(&mut self, text: MessageText) -> u8 {
        let id = self.next_id;
        self.slots[usize::from(id) % HISTORY_SIZE] = Some(Sent { id, text });
        self.next_id = id.wrapping_add(1);
        self.sent = self.sent.saturating_add(1);
        id
    }

    /// Identifier of the most recent message, if any was sent.
    pub fn last_sent(&self) -> Option<u8> {
        (self.sent > 0).then(|| self.next_id.wrapping_sub(1))
    }

    fn get(&self, id: u8) -> Option<&MessageText> {
        self.slots[usize::from(id) % HISTORY_SIZE]
            .as_ref()
            .filter(|s| s.id == id)
            .map(|s| &s.text)
    }

    /// True when the peer claims to have received a message newer than the
    /// last one sent. Such reports MUST be ignored.
    pub fn report_is_ahead(&self, report: MissingMessages) -> bool {
        match self.last_sent() {
            Some(last_sent) => (1..=127).contains(&report.last.wrapping_sub(last_sent)),
            None => false,
        }
    }

    /// Messages that must be retransmitted according to the peer's `report`,
    /// oldest first.
    ///
    /// A message is missing if its bit is `0`, or if it is newer than the
    /// reported `last` and was already sent. Only the last 64 messages sent
    /// can be retransmitted; older ones are lost for good.
    pub fn missing(&self, report: MissingMessages) -> Vec<(u8, &MessageText)> {
        let Some(last_sent) = self.last_sent() else {
            return Vec::new();
        };
        if self.report_is_ahead(report) {
            return Vec::new();
        }

        let depth = self.sent.min(HISTORY_SIZE as u64) as u8;
        (0..depth)
            .rev() // oldest first
            .filter_map(|age| {
                let id = last_sent.wrapping_sub(age);
                let forward = id.wrapping_sub(report.last);
                let missing = match forward {
                    0 => false,       // the reported `last` is received by definition
                    1..=128 => true, // newer than `last` and already sent
                    _ => {
                        // Older than `last`: look up bit `last - id - 1`.
                        let bit = 255 - forward;
                        bit < 64 && (report.received >> bit) & 1 == 0
                    }
                };
                if missing {
                    self.get(id).map(|text| (id, text))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> MessageText {
        MessageText::new(s).unwrap()
    }

    fn history_with(n: u32) -> SendHistory {
        let mut h = SendHistory::new();
        for i in 0..n {
            h.record(text(&format!("m{i}")));
        }
        h
    }

    fn ids(v: Vec<(u8, &MessageText)>) -> Vec<u8> {
        v.into_iter().map(|(id, _)| id).collect()
    }

    // ---- ReceiveWindow ---------------------------------------------------

    #[test]
    fn first_message_is_new_and_keeps_pre_stream_bits_set() {
        let mut w = ReceiveWindow::new();
        assert_eq!(w.accept(0), Arrival::New { distance: 1 });
        assert_eq!(w.report(), MissingMessages { last: 0, received: u64::MAX });
    }

    #[test]
    fn rfc_received_messages_example() {
        let missing = [13u8, 40, 48, 49, 50, 53];
        let mut w = ReceiveWindow::new();
        for id in (0..=52u8).filter(|id| !missing.contains(id)) {
            w.accept(id);
        }
        w.accept(54);
        assert_eq!(
            w.report(),
            MissingMessages {
                last: 54,
                received: 0xFFFF_FEFF_FFFF_DFC6
            }
        );
    }

    #[test]
    fn hole_then_late_arrival_then_duplicate() {
        let mut w = ReceiveWindow::new();
        for id in 0..=4 {
            w.accept(id);
        }
        assert_eq!(w.accept(6), Arrival::New { distance: 2 });
        assert_eq!(w.report().last, 6);
        assert_eq!(w.report().received & 1, 0, "bit 0 is message 5");
        assert_eq!(w.report().received & 0b10, 0b10, "message 4 was received");

        assert_eq!(w.accept(5), Arrival::Late);
        assert_eq!(w.report().received & 1, 1);
        assert_eq!(w.accept(5), Arrival::Ignored);
        assert_eq!(w.accept(6), Arrival::Ignored);
    }

    #[test]
    fn large_jumps_clear_the_whole_window() {
        let mut w = ReceiveWindow::new();
        w.accept(0);
        assert_eq!(w.accept(64), Arrival::New { distance: 64 });
        // old `last` (0) is exactly 64 behind: bit 63.
        assert_eq!(w.report().received, 1 << 63);
        assert_eq!(w.accept(64 + 70), Arrival::New { distance: 70 });
        assert_eq!(w.report().received, 0);
    }

    #[test]
    fn messages_older_than_the_window_are_ignored() {
        let mut w = ReceiveWindow::new();
        for id in 0..=100u8 {
            w.accept(id);
        }
        // 100 - 30 = 70 behind: outside the 64-message window.
        assert_eq!(w.accept(30), Arrival::Ignored);
        // 56 behind and already received.
        assert_eq!(w.accept(44), Arrival::Ignored);
    }

    #[test]
    fn distance_of_exactly_127_is_new_and_128_is_not() {
        let mut w = ReceiveWindow::new();
        w.accept(0);
        assert_eq!(w.accept(127), Arrival::New { distance: 127 });
        let mut w = ReceiveWindow::new();
        w.accept(0);
        // 128 ahead is treated as 128 behind: outside the window.
        assert_eq!(w.accept(128), Arrival::Ignored);
    }

    #[test]
    fn identifiers_wrap_around() {
        let mut w = ReceiveWindow::new();
        for id in 0..=255u8 {
            assert!(matches!(w.accept(id), Arrival::New { distance: 1 }));
        }
        assert_eq!(w.report().last, 255);
        assert_eq!(w.accept(0), Arrival::New { distance: 1 });
        assert_eq!(w.accept(2), Arrival::New { distance: 2 });
        assert_eq!(w.accept(1), Arrival::Late);
    }

    // ---- SendHistory -----------------------------------------------------

    #[test]
    fn nothing_sent_nothing_missing() {
        let h = SendHistory::new();
        assert!(h.missing(MissingMessages::INITIAL).is_empty());
        assert!(h
            .missing(MissingMessages { last: 5, received: 0 })
            .is_empty());
    }

    #[test]
    fn counter_starts_at_zero_and_wraps() {
        let mut h = SendHistory::new();
        assert_eq!(h.last_sent(), None);
        assert_eq!(h.record(text("a")), 0);
        assert_eq!(h.record(text("b")), 1);
        let h = history_with(257);
        assert_eq!(h.last_sent(), Some(0));
    }

    #[test]
    fn unacknowledged_messages_are_missing() {
        let h = history_with(3);
        assert_eq!(ids(h.missing(MissingMessages::INITIAL)), vec![0, 1, 2]);
    }

    #[test]
    fn rfc_lost_last_message_is_found_via_newer_than_last() {
        let h = history_with(8); // 0..=7
        let report = MissingMessages { last: 6, received: u64::MAX };
        assert_eq!(ids(h.missing(report)), vec![7]);
    }

    #[test]
    fn hole_in_bitmap_is_found() {
        let h = history_with(10); // 0..=9
        // last = 9, message 5 is bit 9 - 1 - 5 = 3.
        let report = MissingMessages { last: 9, received: !(1u64 << 3) };
        assert_eq!(ids(h.missing(report)), vec![5]);
    }

    #[test]
    fn retransmits_oldest_first() {
        let h = history_with(20); // 0..=19
        let report = MissingMessages { last: 15, received: !(1u64 << 11) }; // id 3
        assert_eq!(ids(h.missing(report)), vec![3, 16, 17, 18, 19]);
    }

    #[test]
    fn fully_acknowledged_means_nothing_to_resend() {
        let h = history_with(10);
        let report = MissingMessages { last: 9, received: u64::MAX };
        assert!(h.missing(report).is_empty());
    }

    #[test]
    fn report_ahead_of_what_was_sent_is_ignored() {
        let h = history_with(3); // last_sent = 2
        let report = MissingMessages { last: 10, received: 0 };
        assert!(h.report_is_ahead(report));
        assert!(h.missing(report).is_empty());
    }

    #[test]
    fn only_the_last_64_messages_can_be_retransmitted() {
        let h = history_with(200); // last_sent = 199
        let report = MissingMessages { last: 100, received: u64::MAX };
        let expected: Vec<u8> = (136..=199).collect();
        assert_eq!(ids(h.missing(report)), expected);
    }

    #[test]
    fn modular_arithmetic_across_the_wrap() {
        let h = history_with(300); // ids wrapped; last_sent = 299 % 256 = 43
        assert_eq!(h.last_sent(), Some(43));
        let report = MissingMessages { last: 40, received: u64::MAX };
        assert_eq!(ids(h.missing(report)), vec![41, 42, 43]);

        // message 250 (before the wrap) reported missing: 40 - 1 - n = 250.
        let n = (40u8.wrapping_sub(1).wrapping_sub(250)) as u32; // 45
        let report = MissingMessages { last: 40, received: !(1u64 << n) };
        assert_eq!(ids(h.missing(report)), vec![250, 41, 42, 43]);
    }

    #[test]
    fn bits_older_than_the_stream_are_never_retransmitted() {
        let h = history_with(2); // 0, 1
        // Bits for ids before 0 are 0 here, but nothing was ever sent there.
        let report = MissingMessages { last: 1, received: 0 };
        assert_eq!(ids(h.missing(report)), vec![0]);
    }
}
