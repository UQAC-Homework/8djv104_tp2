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

    fn write_variable_u8(&mut self, value: u8, size: usize) {
        assert!(size <= 8, "Size cannot exceed 8 bits");

        if !self.can_write(size) {
            panic!("Cannot write {size} bits to the buffer.")
        }

        let bit_offset = self.offset % 8;
        let position = self.offset / 8;

        // If on two bytes, split
        if bit_offset + size > 8 {
            let overflow = (bit_offset + size) - 8;

            self.buffer[position] |= value >> overflow;
            self.buffer[position + 1] |= value << (8 - overflow);
        }
        // Align on current byte
        else {
            let shift = 8 - bit_offset - size;
            self.buffer[position] |= value << shift;
        }

        self.offset += size;
    }

    fn write_variable_u16(&mut self, value: u16, size: usize) {
        assert!(size <= 16, "Size cannot exceed 16 bits");

        if size <= 8 {
            self.write_variable_u8(value as u8, size);
        } else if size == 16 {
            let upper = value >> 8;
            let lower = value & 0xFF;

            self.write_byte(upper as u8);
            self.write_byte(lower as u8);
        } else {
            let remaining_size = size - (size % 8);
            let upper = value >> remaining_size;

            self.write_byte(upper as u8);
            self.write_variable_u8(upper as u8, remaining_size);
        }

        // If less than a byte, write as is
        if size <= 8 {
            self.write_variable_u8(value as u8, size);
        }
        // If exactly two bytes, write each part
        else if size == 16 {
            self.write_byte((value >> 8) as u8);
            self.write_byte((value & 0xFF) as u8);
        } else {
            let upper_bits = size - 8;
            let upper_val = (value >> 8) as u8;
            let lower_val = (value & 0xFF) as u8;

            self.write_variable_u8(upper_val & ((1 << upper_bits) - 1), upper_bits);
            self.write_byte(lower_val);
        }
    }

    /// Checks if the writer can write the given amount of bits
    fn can_write(&self, size: usize) -> bool {
        self.offset + size <= self.length
    }

    /// Writes the given nybble
    pub fn write_nybble(&mut self, value: u8) {
        self.write_variable_u8(value & 0b1111, 4);
    }

    /// Writes the given 6 bits
    pub fn write_sextet(&mut self, value: u8) {
        self.write_variable_u8(value & 0b111111, 6);
    }

    /// Writes the given byte
    pub fn write_byte(&mut self, value: u8) {
        self.write_variable_u8(value, 8);
    }

    /// Writes the given 10 bits
    pub fn write_declet(&mut self, value: u16) {
        self.write_variable_u16(value, 10);
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
