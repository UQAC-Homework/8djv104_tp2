use std::ops::{AddAssign, Div, Rem};

pub fn write_variable_u8(buffer: &mut [u8; 1024], offset: &mut usize, value: u8, size: usize) {
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

    offset.add_assign(8);
}

pub fn write_variable_u16(buffer: &mut [u8; 1024], offset: &mut usize, value: u16, size: usize) {
    assert!(size <= 16, "Size cannot exceed 16 bits");

    if size <= 8 {
        write_variable_u8(buffer, offset, value as u8, size);
    } else if size == 16 {
        let upper = value >> 8;
        let lower = value & 0xFF;

        write_variable_u8(buffer, offset, upper as u8, 8);
        write_variable_u8(buffer, offset, lower as u8, 8);
    } else {
        let remaining_size = size - (size % 8);
        let upper = value >> remaining_size;

        write_variable_u8(buffer, offset, upper as u8, 8);
        write_variable_u8(buffer, offset, upper as u8, remaining_size);
    }
}
