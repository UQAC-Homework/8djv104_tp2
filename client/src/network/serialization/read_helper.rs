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
        let mask = u8::MAX >> (8 - size);

        value = (buffer[position] >> shift) & mask;
    }

    offset.add_assign(size);
    value
}

/// Reads the given bits from the buffer
fn read_variable_chunks(buffer: &[u8], offset: &mut usize, size: usize) -> u128 {
    let mut value: u128 = 0;
    let full_bytes = size / 8;
    let remainder = size % 8;

    for _ in 0..full_bytes {
        let byte = read_variable_u8(buffer, offset, 8) as u128;
        value = (value << 8) | byte;
    }

    if remainder > 0 {
        let rem_val = read_variable_u8(buffer, offset, remainder) as u128;
        value = (value << remainder) | rem_val;
    }

    value
}

/// Reads the given bits from the buffer
pub fn read_variable_u16(buffer: &[u8], offset: &mut usize, size: usize) -> u16 {
    assert!(size <= 16, "Size cannot exceed 16 bits");
    read_variable_chunks(buffer, offset, size) as u16
}

/// Reads the given bits from the buffer
pub fn read_variable_u32(buffer: &[u8], offset: &mut usize, size: usize) -> u32 {
    assert!(size <= 32, "Size cannot exceed 32 bits");
    read_variable_chunks(buffer, offset, size) as u32
}

/// Reads the given bits from the buffer
pub fn read_variable_u64(buffer: &[u8], offset: &mut usize, size: usize) -> u64 {
    assert!(size <= 64, "Size cannot exceed 64 bits");
    read_variable_chunks(buffer, offset, size) as u64
}