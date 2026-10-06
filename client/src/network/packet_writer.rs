/// Structure used to write packets into a binary format
pub struct PacketWriter {
    /// Buffer to write to
    buffer: [u8; 1024],

    /// Length of the buffer in bits
    length: usize,

    /// Offset in bits
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
        if !self.can_write(4) {
            panic!("Cannot write a nybble to the buffer.")
        }

        // Truncate the input
        let value = value & 0b1111;

        let remaining_bits = self.offset % 4;
        let is_upper = self.offset % 8 == 0;
        let position = self.offset / 8;

        // If on two bytes, split
        if remaining_bits > 0 {
            let bits_to_write = 4 - remaining_bits;

            self.buffer[position] |= value >> bits_to_write;
            self.buffer[position + 1] |= value << bits_to_write;
        }
        // If on upper of byte, shift
        else if is_upper {
            self.buffer[position] |= value << 4;
        }
        // If on lower of byte, write
        else {
            self.buffer[position] |= value;
        }

        self.offset += 4;
    }

    /// Writes the given byte
    pub fn write_byte(&mut self, value: u8) {
        if !self.can_write(8) {
            panic!("Cannot write a byte to the buffer.")
        }

        let remaining_bits = self.offset % 8;
        let position = self.offset / 8;

        // If on two bytes, split
        if remaining_bits > 0 {
            let bits_to_write = 8 - remaining_bits;

            self.buffer[position] |= value >> bits_to_write;
            self.buffer[position + 1] |= value << bits_to_write;
        }
        // If on whole byte, write
        else {
            self.buffer[position] |= value;
        }

        self.offset += 8;
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
