#include "../../include/serialization/ReadHelper.h"

#include <stdexcept>

uint8_t readVariableU8(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	if (size > 8)
		throw std::invalid_argument("Size cannot exceed 8 bits");

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

static uint64_t readVariableChunks(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	uint64_t value = 0;
	const std::size_t fullBytes = size / 8;
	const std::size_t remainder = size % 8;

	for (std::size_t i = 0; i < fullBytes; ++i)
	{
		const uint64_t byte = readVariableU8(buffer, offset, 8);
		value = (value << 8) | byte;
	}

	if (remainder > 0)
	{
		const uint64_t remVal = readVariableU8(buffer, offset, remainder);
		value = (value << remainder) | remVal;
	}

	return value;
}

uint16_t readVariableU16(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	if (size > 16)
		throw std::invalid_argument("Size cannot exceed 16 bits");

	return static_cast<uint16_t>(readVariableChunks(buffer, offset, size));
}

uint32_t readVariableU32(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	if (size > 32)
		throw std::invalid_argument("Size cannot exceed 32 bits");

	return static_cast<uint32_t>(readVariableChunks(buffer, offset, size));
}

uint64_t readVariableU64(const uint8_t* buffer, std::size_t& offset, const std::size_t size)
{
	if (size > 64)
		throw std::invalid_argument("Size cannot exceed 64 bits");

	return readVariableChunks(buffer, offset, size);
}
