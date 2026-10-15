use crate::network::serialization::write_helper::{write_variable_u8, write_variable_u16};

/// Structure used to write packets into a binary format
pub struct PacketWriter {
    /// Buffer to write to
    buffer: [u8; 1024],

    /// Length of the buffer in bits
    length: usize,

    /// Offset from the start of the buffer in bits
    offset: usize,
}

impl PacketWriter {
    /// Creates a new writer
    pub fn new() -> Self {
        Self {
            buffer: [0; 1024],
            length: 1024 * 8,
            offset: 0,
        }
    }

    /// Gets the buffer of the writer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..self.length / 8]
    }

    /// Checks if the writer can write the given amount of bits
    fn can_write(&self, size: usize) -> bool {
        self.offset + size <= self.length
    }

    /// Writes the given nybble
    pub fn write_nybble(&mut self, value: u8) {
        write_variable_u8(&mut self.buffer, &mut self.offset, value & 0b1111, 4);
    }

    /// Writes the given 6 bits
    pub fn write_sextet(&mut self, value: u8) {
        write_variable_u8(&mut self.buffer, &mut self.offset, value & 0b111111, 6);
    }

    /// Writes the given byte
    pub fn write_byte(&mut self, value: u8) {
        write_variable_u8(&mut self.buffer, &mut self.offset, value, 8);
    }

    /// Writes the given 10 bits
    pub fn write_declet(&mut self, value: u16) {
        write_variable_u16(&mut self.buffer, &mut self.offset, value, 10);
    }

    /// Writes the given quadword
    pub fn write_quadword(&mut self, value: u64) {
        todo!()
    }

    /// Writes the given string
    pub fn write_string(&mut self, s: &str) {
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
