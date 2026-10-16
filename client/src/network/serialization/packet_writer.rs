use crate::network::serialization::write_helper::{write_variable_u8, write_variable_u16, write_variable_u64};

/// Maximum size of a packet in bytes
const MAXIMUM_PACKET_SIZE: usize = 1024;

/// Structure used to write packets into a binary format
pub struct PacketWriter {
    /// Buffer to write to
    buffer: Vec<u8>,

    /// Offset from the start of the buffer in bits
    offset: usize,
}

impl PacketWriter {
    /// Creates a new writer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
        }
    }

    /// Gets the buffer of the writer
    pub fn buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    /// Checks if the writer can write the given amount of bits
    fn can_write(&self, size: usize) -> bool {
        self.offset + size <= MAXIMUM_PACKET_SIZE * 8
    }

    /// Writes the given nybble
    pub fn write_nybble(&mut self, value: u8) {
        if !self.can_write(4) {
            panic!("Cannot write a nybble to the buffer.");
        }

        write_variable_u8(&mut self.buffer, &mut self.offset, value & 0b1111, 4);
    }

    /// Writes the given 6 bits
    pub fn write_sextet(&mut self, value: u8) {
        if !self.can_write(6) {
            panic!("Cannot write a sextet to the buffer.");
        }

        write_variable_u8(&mut self.buffer, &mut self.offset, value & 0b111111, 6);
    }

    /// Writes the given byte
    pub fn write_byte(&mut self, value: u8) {
        if !self.can_write(8) {
            panic!("Cannot write a byte to the buffer.");
        }

        write_variable_u8(&mut self.buffer, &mut self.offset, value, 8);
    }

    /// Writes the given 10 bits
    pub fn write_declet(&mut self, value: u16) {
        if !self.can_write(10) {
            panic!("Cannot write a declet to the buffer.");
        }

        write_variable_u16(&mut self.buffer, &mut self.offset, value, 10);
    }

    /// Writes the given quadword
    pub fn write_quadword(&mut self, value: u64) {
        if !self.can_write(64) {
            panic!("Cannot write a quadword to the buffer.");
        }

        write_variable_u64(&mut self.buffer, &mut self.offset, value, 64);
    }

    /// Writes the given string
    pub fn write_string(&mut self, s: &String) {
        let value_length = s.len();

        if !self.can_write(value_length * 8) {
            panic!("Cannot write a string of {value_length} characters to the buffer.")
        }

        let bytes = s.as_bytes();

        for byte in bytes {
            self.write_byte(*byte);
        }
    }
}
