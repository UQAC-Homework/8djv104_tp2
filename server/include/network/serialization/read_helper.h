#ifndef SERVER_READ_HELPER_H
#define SERVER_READ_HELPER_H
#include <cassert>
#include <cstdint>

/// Reads the given bits from the buffer
inline uint8_t readVariableU8(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	assert(size > 0);
	assert(size <= 8);

	const std::size_t bitOffset = offset % 8;
	const std::size_t position = offset / 8;

	uint8_t value;

	// If on two bytes, merge
	if (bitOffset + size > 8)
	{
		const std::size_t overflow = bitOffset + size - 8;

		const auto upperPart = static_cast<uint8_t>(
			(buffer[position] & (1u << (8 - bitOffset)) - 1) << overflow);
		const auto lowerPart = static_cast<uint8_t>(
			buffer[position + 1] >> (16 - bitOffset - size));

		value = upperPart | lowerPart;
	}
	// Extract from current byte
	else
	{
		const std::size_t shift = 8 - bitOffset - size;
		const auto mask = static_cast<uint8_t>(0xFFu >> (8 - size));

		value = (buffer[position] >> shift) & mask;
	}

	offset += size;
	return value;
}

/// Reads the given bits from the buffer
inline uint16_t readVariableU16(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	assert(size <= 16);

	uint16_t value = 0;

	if (size == 16)
	{
		const auto upper = readVariableU8(buffer, offset, 8);
		const auto lower = readVariableU8(buffer, offset, 8);

		value = upper << 8 | lower;
	}
	else if (size > 8)
	{
		const auto remainder = size % 8;
		const auto upper = readVariableU8(buffer, offset, 8);
		const auto lower = readVariableU8(buffer, offset, remainder);

		value = static_cast<uint16_t>(upper) << remainder | lower;
	}
	else
		value = readVariableU8(buffer, offset, size);

	return value;
}

/// Reads the given bits from the buffer
inline uint32_t readVariableU32(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	assert(size <= 32);

	uint32_t value = 0;

	if (size == 32)
	{
		const auto upper = readVariableU16(buffer, offset, 16);
		const auto lower = readVariableU16(buffer, offset, 16);

		value = upper << 16 | lower;
	}
	else if (size > 16)
	{
		const auto remainder = size % 16;
		const auto upper = readVariableU16(buffer, offset, 16);
		const auto lower = readVariableU16(buffer, offset, remainder);

		value = static_cast<uint32_t>(upper) << remainder | lower;
	}
	else
		value = readVariableU16(buffer, offset, size);

	return value;
}

/// Reads the given bits from the buffer
inline uint64_t readVariableU64(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	assert(size <= 64);

	uint64_t value = 0;

	if (size == 64)
	{
		const auto upper = readVariableU32(buffer, offset, 32);
		const auto lower = readVariableU32(buffer, offset, 32);

		value = static_cast<uint64_t>(upper) << 32 | lower;
	}
	else if (size > 32)
	{
		const auto remainder = size % 32;
		const auto upper = readVariableU32(buffer, offset, 32);
		const auto lower = readVariableU32(buffer, offset, remainder);

		value = static_cast<uint64_t>(upper) << remainder | lower;
	}
	else
		value = readVariableU32(buffer, offset, size);

	return value;
}

#endif //SERVER_READ_HELPER_H
