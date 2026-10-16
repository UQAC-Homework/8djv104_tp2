use crate::network::serialization::read_helper::{read_variable_u8, read_variable_u16};

/// Structure used to read packets from a binary format
pub struct PacketReader<'a> {
    /// Buffer to read from
    buffer: &'a [u8],

    /// Length of the buffer in bits
    length: usize,

    /// Offset from the start of the buffer in bits
    offset: usize,
}

impl<'buf> PacketReader<'buf> {
    /// Creates a new reader from a byte slice
    pub fn new(buffer: &'buf [u8]) -> Self {
        Self {
            buffer,
            length: buffer.len() * 8,
            offset: 0,
        }
    }

    /// Checks if the reader can read the given amount of bits
    fn can_read(&self, size: usize) -> bool {
        self.offset + size <= self.length
    }

    /// Reads the given nybble
    pub fn read_nybble(&mut self) -> u8 {
        if !self.can_read(4) {
            panic!("Cannot read a nybble from the buffer.");
        }

        read_variable_u8(self.buffer, &mut self.offset, 4) & 0b1111
    }

    /// Reads the given 6 bits
    pub fn read_sextet(&mut self) -> u8 {
        if !self.can_read(6) {
            panic!("Cannot read a sextet from the buffer.");
        }

        read_variable_u8(self.buffer, &mut self.offset, 6) & 0b111111
    }

    /// Reads the given byte
    pub fn read_byte(&mut self) -> u8 {
        if !self.can_read(8) {
            panic!("Cannot read a byte from the buffer.");
        }

        read_variable_u8(self.buffer, &mut self.offset, 8)
    }

    /// Reads the given 10 bits
    pub fn read_declet(&mut self) -> u16 {
        if !self.can_read(10) {
            panic!("Cannot read a declet from the buffer.");
        }
        read_variable_u16(self.buffer, &mut self.offset, 10)
    }

    /// Reads the given quadword
    pub fn read_quadword(&mut self) -> u64 {
        todo!()
    }

    /// Reads the given string of a specified character length
    pub fn read_string(&mut self, length: usize) -> String {
        if !self.can_read(length * 8) {
            panic!("Cannot read a string of {length} characters from the buffer.");
        }

        let mut bytes = Vec::with_capacity(length);

        for _ in 0..length {
            bytes.push(self.read_byte());
        }

        String::from_utf8(bytes).unwrap_or_else(|_| panic!("Failed to parse UTF-8 string."))
    }
}
