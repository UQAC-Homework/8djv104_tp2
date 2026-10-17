use crate::network::serialization::errors::DecodeError;
use crate::network::serialization::read_helper::{
    read_variable_u8, read_variable_u16, read_variable_u64,
};

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
    pub fn read_nybble(&mut self) -> Result<u8, DecodeError> {
        if !self.can_read(4) {
            return Err(DecodeError::NotEnoughData(4));
        }

        let value = read_variable_u8(self.buffer, &mut self.offset, 4)?;
        Ok(value & 0b1111)
    }

    /// Reads the given 6 bits
    pub fn read_sextet(&mut self) -> Result<u8, DecodeError> {
        if !self.can_read(6) {
            return Err(DecodeError::NotEnoughData(6));
        }

        let value = read_variable_u8(self.buffer, &mut self.offset, 6)?;
        Ok(value & 0b111111)
    }

    /// Reads the given byte
    pub fn read_byte(&mut self) -> Result<u8, DecodeError> {
        if !self.can_read(8) {
            return Err(DecodeError::NotEnoughData(8));
        }

        read_variable_u8(self.buffer, &mut self.offset, 8)
    }

    /// Reads the given 10 bits
    pub fn read_declet(&mut self) -> Result<u16, DecodeError> {
        if !self.can_read(10) {
            return Err(DecodeError::NotEnoughData(10));
        }

        let value = read_variable_u16(self.buffer, &mut self.offset, 10)?;
        Ok(value & 0b1111111111)
    }

    /// Reads the given quadword
    pub fn read_quadword(&mut self) -> Result<u64, DecodeError> {
        if !self.can_read(64) {
            return Err(DecodeError::NotEnoughData(64));
        }

        read_variable_u64(self.buffer, &mut self.offset, 64)
    }

    /// Reads the given string of a specified character length
    pub fn read_string(&mut self, length: usize) -> Result<String, DecodeError> {
        if !self.can_read(length * 8) {
            return Err(DecodeError::NotEnoughData(length * 8));
        }

        let mut bytes = Vec::with_capacity(length);

        for _ in 0..length {
            bytes.push(self.read_byte()?);
        }

        Ok(String::from_utf8(bytes).unwrap_or_else(|_| panic!("Failed to parse UTF-8 string.")))
    }
}
