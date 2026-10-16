use std::ops::{AddAssign, Div, Rem};

/// Reads the given bits from the buffer
pub fn read_variable_u8(buffer: &[u8], offset: &mut usize, size: usize) -> u8 {
    assert!(size <= 8, "Size cannot exceed 8 bits");

    let bit_offset = offset.rem(8);
    let position = offset.div(8);

    let value: u8;

    // If on two bytes, merge
    if bit_offset + size > 8 {
        let overflow = (bit_offset + size) - 8;

        let upper_part = (buffer[position] & ((1 << (8 - bit_offset)) - 1)) << overflow;
        let lower_part = buffer[position + 1] >> (16 - bit_offset - size);

        value = upper_part | lower_part;
    }
    // Extract from current byte
    else {
        let shift = 8 - bit_offset - size;
        value = (buffer[position] >> shift) & ((1 << size) - 1);
    }

    offset.add_assign(size);
    value
}

/// Reads the given bits from the buffer
pub fn read_variable_u16(buffer: &[u8], offset: &mut usize, size: usize) -> u16 {
    assert!(size <= 16, "Size cannot exceed 16 bits");

    if size <= 8 {
        read_variable_u8(buffer, offset, size) as u16
    } else if size == 16 {
        let upper = read_variable_u8(buffer, offset, 8) as u16;
        let lower = read_variable_u8(buffer, offset, 8) as u16;
        (upper << 8) | lower
    } else {
        let remaining_size = size - 8;
        let upper = read_variable_u8(buffer, offset, 8) as u16;
        let lower = read_variable_u8(buffer, offset, remaining_size) as u16;

        (upper << remaining_size) | lower
    }
}
