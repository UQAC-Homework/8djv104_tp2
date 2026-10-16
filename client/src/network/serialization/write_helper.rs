use std::ops::{AddAssign, Div, Rem};

/// Writes the given bits into the buffer
pub fn write_variable_u8(buffer: &mut [u8], offset: &mut usize, value: u8, size: usize) {
    assert!(size <= 8, "Size cannot exceed 8 bits");

    let bit_offset = offset.rem(8);
    let position = offset.div(8);

    // If on two bytes, split
    if bit_offset + size > 8 {
        let overflow = (bit_offset + size) - 8;

        buffer[position] |= value >> overflow;
        buffer[position + 1] |= value << (8 - overflow);
    }
    // Align on current byte
    else {
        let shift = 8 - bit_offset - size;
        buffer[position] |= value << shift;
    }

    offset.add_assign(size);
}

/// Writes the given bits into the buffer
fn write_variable_chunks(buffer: &mut [u8], offset: &mut usize, value: u128, size: usize) {
    let full_bytes = size / 8;
    let remainder = size % 8;

    // Write each full byte (MSB first)
    for i in (0..full_bytes).rev() {
        let chunk = ((value >> (i * 8 + remainder)) & 0xFF) as u8;
        write_variable_u8(buffer, offset, chunk, 8);
    }

    // Write remaining bits
    if remainder > 0 {
        let chunk = (value & ((1 << remainder) - 1)) as u8;
        write_variable_u8(buffer, offset, chunk, remainder);
    }
}

/// Writes the given bits into the buffer
pub fn write_variable_u16(buffer: &mut [u8], offset: &mut usize, value: u16, size: usize) {
    assert!(size <= 16, "Size cannot exceed 16 bits");
    write_variable_chunks(buffer, offset, value as u128, size);
}

/// Writes the given bits into the buffer
pub fn write_variable_u32(buffer: &mut [u8], offset: &mut usize, value: u32, size: usize) {
    assert!(size <= 32, "Size cannot exceed 32 bits");
    write_variable_chunks(buffer, offset, value as u128, size);
}

/// Writes the given bits into the buffer
pub fn write_variable_u64(buffer: &mut [u8], offset: &mut usize, value: u64, size: usize) {
    assert!(size <= 64, "Size cannot exceed 64 bits");
    write_variable_chunks(buffer, offset, value as u128, size);
}
