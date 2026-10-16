//! Wire format of the chat protocol (RFC sections "Headers" and "Packets").
//!
//! Every packet is a bit-packed record, most significant bit first
//! ("big-endian"). Fields are generally *not* byte aligned (user identifiers
//! are 6 bits, message lengths 10 bits, ...) and the variable-length
//! `Username` / `Message` fields shift everything that follows them, so the
//! codec works on a bit cursor instead of fixed byte offsets. The last byte
//! of a packet is padded with zero bits.

use std::fmt;

/// Longest username, in bytes (`Username Length` is 4 bits, offset by one).
pub const MAX_USERNAME_LEN: usize = 16;
/// Longest message, in bytes (RFC: `Message Length` MUST NOT exceed 990).
pub const MAX_MESSAGE_LEN: usize = 990;

mod ty {
    pub const CONNECTION_REQUEST: u8 = 0x01;
    pub const CONNECTION_ACCEPTED: u8 = 0x02;
    pub const CONNECTION_REJECTED: u8 = 0x03;
    pub const CONNECTION_CLOSED: u8 = 0x04;
    pub const MESSAGE_SUBMIT: u8 = 0x10;
    pub const MESSAGE_BROADCAST: u8 = 0x11;
    pub const MESSAGE_REQUEST: u8 = 0x12;
    pub const HEARTBEAT_BROADCAST: u8 = 0x20;
    pub const HEARTBEAT_RECEIVED: u8 = 0x21;
    pub const HEARTBEAT_REQUEST: u8 = 0x22;
}

// ---------------------------------------------------------------------------
// Validated field types
// ---------------------------------------------------------------------------

/// Why a username or message was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    Empty,
    NotAscii,
    TooLong { max: usize },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldError::Empty => write!(f, "must not be empty"),
            FieldError::NotAscii => write!(f, "must contain ASCII characters only"),
            FieldError::TooLong { max } => write!(f, "must be at most {max} characters"),
        }
    }
}

impl std::error::Error for FieldError {}

fn check_text(s: &str, max: usize) -> Result<(), FieldError> {
    if s.is_empty() {
        Err(FieldError::Empty)
    } else if !s.is_ascii() {
        Err(FieldError::NotAscii)
    } else if s.len() > max {
        Err(FieldError::TooLong { max })
    } else {
        Ok(())
    }
}

/// A username: 1 to 16 ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub fn new(name: impl Into<String>) -> Result<Self, FieldError> {
        let name = name.into();
        check_text(&name, MAX_USERNAME_LEN)?;
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A chat message: 1 to 990 ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageText(String);

impl MessageText {
    pub fn new(text: impl Into<String>) -> Result<Self, FieldError> {
        let text = text.into();
        check_text(&text, MAX_MESSAGE_LEN)?;
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MessageText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Header `MISSING_MESSAGES`: what the sender of a packet has received from
/// the receiver of that packet.
///
/// Bit `n` of `received` (least significant bit first) is set when the
/// message with identifier `last - 1 - n` (modulo 256) was received. `last`
/// itself is always considered received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingMessages {
    pub last: u8,
    pub received: u64,
}

impl MissingMessages {
    /// Initial value on both sides: nothing received yet.
    pub const INITIAL: Self = Self {
        last: 0xFF,
        received: u64::MAX,
    };
}

// ---------------------------------------------------------------------------
// Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    /// 0x01, client -> server.
    ConnectionRequest { client_id: u8, username: Username },
    /// 0x02, server -> client.
    ConnectionAccepted { user_id: u8 },
    /// 0x03, server -> client.
    ConnectionRejected,
    /// 0x04, both directions. A server shutting down uses `user_id == 0`.
    ConnectionClosed { user_id: u8 },
    /// 0x10, client -> server.
    MessageSubmit {
        user_id: u8,
        report: MissingMessages,
        message_id: u8,
        message: MessageText,
    },
    /// 0x11, server -> client.
    MessageBroadcast {
        message_id: u8,
        report: MissingMessages,
        username: Username,
        message: MessageText,
    },
    /// 0x12, both directions.
    MessageRequest {
        user_id: u8,
        report: MissingMessages,
    },
    /// 0x20, client -> server.
    HeartbeatBroadcast {
        user_id: u8,
        report: MissingMessages,
    },
    /// 0x21, server -> client.
    HeartbeatReceived { report: MissingMessages },
    /// 0x22, server -> client.
    HeartbeatRequest { report: MissingMessages },
}

/// Why a datagram was not turned into a [`Packet`]. In every case the RFC
/// requires the datagram to be discarded ("unsupported type MUST be ignored",
/// "malformed packet MUST be discarded").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The datagram ended before all fields were read.
    Truncated,
    /// Whole bytes remain after the last field.
    TrailingBytes,
    /// `Type` is `0x00`, which the RFC forbids.
    ReservedType,
    /// A type this implementation does not know.
    UnknownType(u8),
    /// A field holds a value the RFC forbids.
    Invalid(&'static str),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "truncated packet"),
            DecodeError::TrailingBytes => write!(f, "unexpected bytes after the last field"),
            DecodeError::ReservedType => write!(f, "packet type 0x00 is reserved"),
            DecodeError::UnknownType(t) => write!(f, "unsupported packet type {t:#04x}"),
            DecodeError::Invalid(what) => write!(f, "invalid {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl Packet {
    /// Serialises the packet into one datagram.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        match self {
            Packet::ConnectionRequest {
                client_id,
                username,
            } => {
                w.put(ty::CONNECTION_REQUEST.into(), 8);
                w.put((client_id & 0x0F).into(), 4);
                put_username(&mut w, username);
            }
            Packet::ConnectionAccepted { user_id } => {
                w.put(ty::CONNECTION_ACCEPTED.into(), 8);
                w.put((user_id & 0x3F).into(), 6);
            }
            Packet::ConnectionRejected => {
                w.put(ty::CONNECTION_REJECTED.into(), 8);
            }
            Packet::ConnectionClosed { user_id } => {
                w.put(ty::CONNECTION_CLOSED.into(), 8);
                w.put((user_id & 0x3F).into(), 6);
            }
            Packet::MessageSubmit {
                user_id,
                report,
                message_id,
                message,
            } => {
                w.put(ty::MESSAGE_SUBMIT.into(), 8);
                w.put((user_id & 0x3F).into(), 6);
                put_report(&mut w, *report);
                w.put((*message_id).into(), 8);
                put_message(&mut w, message);
            }
            Packet::MessageBroadcast {
                message_id,
                report,
                username,
                message,
            } => {
                w.put(ty::MESSAGE_BROADCAST.into(), 8);
                w.put((*message_id).into(), 8);
                put_report(&mut w, *report);
                put_username(&mut w, username);
                put_message(&mut w, message);
            }
            Packet::MessageRequest { user_id, report } => {
                w.put(ty::MESSAGE_REQUEST.into(), 8);
                w.put((user_id & 0x3F).into(), 6);
                put_report(&mut w, *report);
            }
            Packet::HeartbeatBroadcast { user_id, report } => {
                w.put(ty::HEARTBEAT_BROADCAST.into(), 8);
                w.put((user_id & 0x3F).into(), 6);
                put_report(&mut w, *report);
            }
            Packet::HeartbeatReceived { report } => {
                w.put(ty::HEARTBEAT_RECEIVED.into(), 8);
                put_report(&mut w, *report);
            }
            Packet::HeartbeatRequest { report } => {
                w.put(ty::HEARTBEAT_REQUEST.into(), 8);
                put_report(&mut w, *report);
            }
        }
        w.finish()
    }

    /// Parses one datagram.
    pub fn decode(bytes: &[u8]) -> Result<Packet, DecodeError> {
        let mut r = BitReader::new(bytes);
        let packet = match r.take(8)? as u8 {
            0x00 => return Err(DecodeError::ReservedType),
            ty::CONNECTION_REQUEST => Packet::ConnectionRequest {
                client_id: r.take(4)? as u8,
                username: take_username(&mut r)?,
            },
            ty::CONNECTION_ACCEPTED => {
                let user_id = r.take(6)? as u8;
                if user_id == 0 {
                    return Err(DecodeError::Invalid("user identifier"));
                }
                Packet::ConnectionAccepted { user_id }
            }
            ty::CONNECTION_REJECTED => Packet::ConnectionRejected,
            ty::CONNECTION_CLOSED => Packet::ConnectionClosed {
                user_id: r.take(6)? as u8,
            },
            ty::MESSAGE_SUBMIT => Packet::MessageSubmit {
                user_id: r.take(6)? as u8,
                report: take_report(&mut r)?,
                message_id: r.take(8)? as u8,
                message: take_message(&mut r)?,
            },
            ty::MESSAGE_BROADCAST => Packet::MessageBroadcast {
                message_id: r.take(8)? as u8,
                report: take_report(&mut r)?,
                username: take_username(&mut r)?,
                message: take_message(&mut r)?,
            },
            ty::MESSAGE_REQUEST => Packet::MessageRequest {
                user_id: r.take(6)? as u8,
                report: take_report(&mut r)?,
            },
            ty::HEARTBEAT_BROADCAST => Packet::HeartbeatBroadcast {
                user_id: r.take(6)? as u8,
                report: take_report(&mut r)?,
            },
            ty::HEARTBEAT_RECEIVED => Packet::HeartbeatReceived {
                report: take_report(&mut r)?,
            },
            ty::HEARTBEAT_REQUEST => Packet::HeartbeatRequest {
                report: take_report(&mut r)?,
            },
            other => return Err(DecodeError::UnknownType(other)),
        };
        r.finish()?;
        Ok(packet)
    }
}

// ---------------------------------------------------------------------------
// Header encoders / decoders
// ---------------------------------------------------------------------------

/// `MISSING_MESSAGES`: 8 + 64 bits.
fn put_report(w: &mut BitWriter, report: MissingMessages) {
    w.put(report.last.into(), 8);
    w.put(report.received, 64);
}

fn take_report(r: &mut BitReader) -> Result<MissingMessages, DecodeError> {
    Ok(MissingMessages {
        last: r.take(8)? as u8,
        received: r.take(64)?,
    })
}

/// `USERNAME_CONTENT`: 4-bit length (offset by one) + that many bytes.
fn put_username(w: &mut BitWriter, username: &Username) {
    let bytes = username.as_str().as_bytes();
    w.put((bytes.len() - 1) as u64, 4);
    w.put_bytes(bytes);
}

fn take_username(r: &mut BitReader) -> Result<Username, DecodeError> {
    let len = r.take(4)? as usize + 1;
    let bytes = r.take_bytes(len)?;
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| Username::new(s).ok())
        .ok_or(DecodeError::Invalid("username"))
}

/// `MESSAGE_CONTENT`: 10-bit length + that many bytes.
fn put_message(w: &mut BitWriter, message: &MessageText) {
    let bytes = message.as_str().as_bytes();
    w.put(bytes.len() as u64, 10);
    w.put_bytes(bytes);
}

fn take_message(r: &mut BitReader) -> Result<MessageText, DecodeError> {
    let len = r.take(10)? as usize;
    if len == 0 || len > MAX_MESSAGE_LEN {
        return Err(DecodeError::Invalid("message length"));
    }
    let bytes = r.take_bytes(len)?;
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| MessageText::new(s).ok())
        .ok_or(DecodeError::Invalid("message"))
}

// ---------------------------------------------------------------------------
// Bit cursor
// ---------------------------------------------------------------------------

struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bits: 0,
        }
    }

    /// Appends the low `width` bits of `value`, most significant bit first.
    fn put(&mut self, value: u64, width: u32) {
        debug_assert!(width <= 64 && (width == 64 || value >> width == 0));
        for i in (0..width).rev() {
            if self.bits % 8 == 0 {
                self.bytes.push(0);
            }
            if (value >> i) & 1 == 1 {
                *self.bytes.last_mut().expect("byte was just pushed") |= 0x80 >> (self.bits % 8);
            }
            self.bits += 1;
        }
    }

    fn put_bytes(&mut self, data: &[u8]) {
        for &b in data {
            self.put(b.into(), 8);
        }
    }

    /// Trailing bits of the last byte stay zero.
    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len() * 8 - self.pos
    }

    fn take(&mut self, width: u32) -> Result<u64, DecodeError> {
        if self.remaining() < width as usize {
            return Err(DecodeError::Truncated);
        }
        let mut value = 0u64;
        for _ in 0..width {
            let bit = (self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1;
            value = (value << 1) | u64::from(bit);
            self.pos += 1;
        }
        Ok(value)
    }

    fn take_bytes(&mut self, n: usize) -> Result<Vec<u8>, DecodeError> {
        if self.remaining() < n * 8 {
            return Err(DecodeError::Truncated);
        }
        (0..n).map(|_| self.take(8).map(|b| b as u8)).collect()
    }

    /// Only the zero padding of the final byte may remain.
    fn finish(self) -> Result<(), DecodeError> {
        if self.remaining() >= 8 {
            Err(DecodeError::TrailingBytes)
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn user(s: &str) -> Username {
        Username::new(s).unwrap()
    }

    fn text(s: &str) -> MessageText {
        MessageText::new(s).unwrap()
    }

    #[test]
    fn rfc_example_connection_request() {
        let packet = Packet::ConnectionRequest {
            client_id: 0xC,
            username: user("Player"),
        };
        let bytes = [0x01, 0xC5, 0x50, 0x6C, 0x61, 0x79, 0x65, 0x72];
        assert_eq!(packet.encode(), bytes);
        assert_eq!(Packet::decode(&bytes), Ok(packet));
    }

    #[test]
    fn rfc_example_missing_messages_header() {
        let report = MissingMessages {
            last: 54,
            received: 0xFFFF_FEFF_FFFF_DFC6,
        };
        let bytes = Packet::HeartbeatReceived { report }.encode();
        assert_eq!(
            bytes,
            [0x21, 0x36, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0xDF, 0xC6]
        );
        assert_eq!(
            Packet::decode(&bytes),
            Ok(Packet::HeartbeatReceived { report })
        );
    }

    #[test]
    fn username_length_is_offset_by_one() {
        let one = Packet::ConnectionRequest {
            client_id: 0,
            username: user("a"),
        }
        .encode();
        assert_eq!(one, [0x01, 0x00, b'a']);
        let sixteen = Packet::ConnectionRequest {
            client_id: 0,
            username: user("abcdefghijklmnop"),
        }
        .encode();
        assert_eq!(sixteen[1], 0x0F);
    }

    #[test]
    fn small_packets_use_bit_packed_user_ids() {
        // user id occupies bits 8..=13, i.e. the top six bits of byte 1.
        assert_eq!(
            Packet::ConnectionAccepted { user_id: 5 }.encode(),
            [0x02, 5 << 2]
        );
        assert_eq!(
            Packet::ConnectionClosed { user_id: 63 }.encode(),
            [0x04, 0xFC]
        );
        assert_eq!(Packet::ConnectionRejected.encode(), [0x03]);
    }

    #[test]
    fn message_submit_layout() {
        let packet = Packet::MessageSubmit {
            user_id: 0b101010,
            report: MissingMessages {
                last: 0xAB,
                received: 0x0123_4567_89AB_CDEF,
            },
            message_id: 0x7E,
            message: text("Hi"),
        };
        let bytes = packet.encode();
        // 104 header bits (13 bytes) + 2 message bytes.
        assert_eq!(bytes.len(), 15);
        assert_eq!(bytes[0], 0x10);
        assert_eq!(bytes[1] >> 2, 0b101010);
        // last identifier: 2 bits of byte 1, 6 bits of byte 2.
        assert_eq!(((bytes[1] & 0b11) << 6) | (bytes[2] >> 2), 0xAB);
        assert_eq!(Packet::decode(&bytes), Ok(packet));
    }

    #[test]
    fn broadcast_fields_follow_the_actual_username_length() {
        // Username of 3 bytes: the message length starts at bit 92 + 24, not
        // at the bit the diagram shows for a full-size username.
        let packet = Packet::MessageBroadcast {
            message_id: 9,
            report: MissingMessages::INITIAL,
            username: user("Bob"),
            message: text("Hello World!"),
        };
        let bytes = packet.encode();
        let bits = 92 + 24 + 10 + 8 * 12;
        assert_eq!(bytes.len(), (bits + 7) / 8);
        assert_eq!(Packet::decode(&bytes), Ok(packet));
    }

    #[test]
    fn every_packet_round_trips() {
        let report = MissingMessages {
            last: 3,
            received: 0xDEAD_BEEF_0000_FFFF,
        };
        let packets = vec![
            Packet::ConnectionRequest {
                client_id: 15,
                username: user("x"),
            },
            Packet::ConnectionAccepted { user_id: 63 },
            Packet::ConnectionRejected,
            Packet::ConnectionClosed { user_id: 0 },
            Packet::ConnectionClosed { user_id: 12 },
            Packet::MessageSubmit {
                user_id: 1,
                report,
                message_id: 255,
                message: text(&"z".repeat(MAX_MESSAGE_LEN)),
            },
            Packet::MessageBroadcast {
                message_id: 0,
                report,
                username: user(&"u".repeat(MAX_USERNAME_LEN)),
                message: text(&"m".repeat(MAX_MESSAGE_LEN)),
            },
            Packet::MessageBroadcast {
                message_id: 1,
                report,
                username: user("ab"),
                message: text("c"),
            },
            Packet::MessageRequest { user_id: 7, report },
            Packet::HeartbeatBroadcast { user_id: 9, report },
            Packet::HeartbeatReceived { report },
            Packet::HeartbeatRequest { report },
        ];
        for packet in packets {
            let bytes = packet.encode();
            assert_eq!(Packet::decode(&bytes), Ok(packet.clone()), "{packet:?}");
        }
    }

    #[test]
    fn rejects_malformed_packets() {
        assert_eq!(Packet::decode(&[]), Err(DecodeError::Truncated));
        assert_eq!(Packet::decode(&[0x00]), Err(DecodeError::ReservedType));
        assert_eq!(Packet::decode(&[0x7F]), Err(DecodeError::UnknownType(0x7F)));
        // truncated username
        assert_eq!(
            Packet::decode(&[0x01, 0x05, b'a']),
            Err(DecodeError::Truncated)
        );
        // trailing bytes
        assert_eq!(
            Packet::decode(&[0x03, 0x00]),
            Err(DecodeError::TrailingBytes)
        );
        // accepted with the reserved user id 0
        assert_eq!(
            Packet::decode(&[0x02, 0x00]),
            Err(DecodeError::Invalid("user identifier"))
        );
        // non-ASCII username
        assert_eq!(
            Packet::decode(&[0x01, 0x00, 0xC3]),
            Err(DecodeError::Invalid("username"))
        );
    }

    #[test]
    fn rejects_bad_message_lengths_and_content() {
        let good = Packet::MessageRequest {
            user_id: 1,
            report: MissingMessages::INITIAL,
        };
        assert!(Packet::decode(&good.encode()).is_ok());

        let mut w = BitWriter::new();
        w.put(ty::MESSAGE_BROADCAST.into(), 8);
        w.put(0, 8);
        put_report(&mut w, MissingMessages::INITIAL);
        w.put(0, 4);
        w.put_bytes(b"a");
        w.put(0, 10); // message length 0
        assert_eq!(
            Packet::decode(&w.finish()),
            Err(DecodeError::Invalid("message length"))
        );

        let mut w = BitWriter::new();
        w.put(ty::MESSAGE_BROADCAST.into(), 8);
        w.put(0, 8);
        put_report(&mut w, MissingMessages::INITIAL);
        w.put(0, 4);
        w.put_bytes(b"a");
        w.put(991, 10); // longer than allowed
        w.put_bytes(&[b'x'; 991]);
        assert_eq!(
            Packet::decode(&w.finish()),
            Err(DecodeError::Invalid("message length"))
        );
    }

    #[test]
    fn field_validation() {
        assert_eq!(Username::new(""), Err(FieldError::Empty));
        assert_eq!(Username::new("é"), Err(FieldError::NotAscii));
        assert_eq!(
            Username::new("a".repeat(17)),
            Err(FieldError::TooLong { max: 16 })
        );
        assert_eq!(MessageText::new(""), Err(FieldError::Empty));
        assert!(MessageText::new("a".repeat(990)).is_ok());
        assert_eq!(
            MessageText::new("a".repeat(991)),
            Err(FieldError::TooLong { max: 990 })
        );
    }
}
