#ifndef SERVER_WRITE_HELPER_H
#define SERVER_WRITE_HELPER_H
#include <cassert>
#include <cstdint>

/// Writes the given bits into the buffer
inline std::size_t writeVariableU8(uint8_t* buffer, const std::size_t offset, const uint8_t value,
                                   const std::size_t size)
{
	assert(size <= 8);

	const std::size_t bitOffset = offset % 8;
	const std::size_t position = offset / 8;

	//const std::size_t requiredLen = (offset + size + 7) / 8;

	//if (buffer.size() < requiredLen)
	//	buffer.resize(requiredLen, 0);

	// If on two bytes, split
	if (bitOffset + size > 8)
	{
		const std::size_t overflow = bitOffset + size - 8;

		buffer[position] |= static_cast<uint8_t>(value >> overflow);
		buffer[position + 1] |= static_cast<uint8_t>(value << (8 - overflow));
	}
	// Align on current byte
	else
	{
		const std::size_t shift = 8 - bitOffset - size;
		buffer[position] |= static_cast<uint8_t>(value << shift);
	}

	return offset + size;
}

/// Writes the given bits into the buffer
inline std::size_t writeVariableU16(uint8_t* buffer, const std::size_t offset, const uint16_t value,
                                    const std::size_t size)
{
	assert(size <= 16);

	if (size == 16)
	{
		const auto upper = value >> 8;
		const auto lower = value & 0xFF;

		writeVariableU8(buffer, offset, upper, 8);
		writeVariableU8(buffer, offset + 8, lower, 8);
	}
	else if (size > 8)
	{
		const auto remainder = size % 8;
		const auto upper = value >> 8;
		const auto lower = value & 0xFF;

		writeVariableU8(buffer, offset, upper, remainder);
		writeVariableU8(buffer, offset + remainder, lower, 8);
	}
	else
		writeVariableU16(buffer, offset, value, size);

	return offset + size;
}

/// Writes the given bits into the buffer
inline std::size_t writeVariableU32(uint8_t* buffer, const std::size_t offset, const uint32_t value,
                                    const std::size_t size)
{
	assert(size <= 32);

	if (size == 32)
	{
		const auto upper = value >> 16;
		const auto lower = value & 0xFFFF;

		writeVariableU16(buffer, offset, upper, 16);
		writeVariableU16(buffer, offset + 16, lower, 16);
	}
	else if (size > 16)
	{
		const auto remainder = size % 16;
		const auto upper = value >> 16;
		const auto lower = value & 0xFFFF;

		writeVariableU16(buffer, offset, upper, remainder);
		writeVariableU16(buffer, offset + remainder, lower, 16);
	}
	else
		writeVariableU16(buffer, offset, value, size);

	return offset + size;
}

/// Writes the given bits into the buffer
inline std::size_t writeVariableU64(uint8_t* buffer, const std::size_t offset, const uint64_t value,
                                    const std::size_t size)
{
	assert(size <= 64);

	if (size == 64)
	{
		const auto upper = value >> 32;
		const auto lower = value & 0xFFFFFFFF;

		writeVariableU32(buffer, offset, upper, 32);
		writeVariableU32(buffer, offset + 32, lower, 32);
	}
	else if (size > 32)
	{
		const auto remainder = size % 32;
		const auto upper = value >> 32;
		const auto lower = value & 0xFFFFFFFF;

		writeVariableU32(buffer, offset, upper, remainder);
		writeVariableU32(buffer, offset + remainder, lower, 32);
	}
	else
		writeVariableU32(buffer, offset, value, size);

	return offset + size;
}

#endif //SERVER_WRITE_HELPER_H
