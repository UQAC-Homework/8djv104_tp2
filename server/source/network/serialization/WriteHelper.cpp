#include "../../../include/network/serialization/WriteHelper.h"

#include <stdexcept>

void writeVariableU8(std::vector<uint8_t>& buffer, std::size_t& offset, const uint8_t value, const std::size_t size)
{
	if (size > 8)
		throw std::invalid_argument("Size cannot exceed 8 bits");

	const std::size_t bitOffset = offset % 8;
	const std::size_t position = offset / 8;

	const std::size_t requiredLen = (offset + size + 7) / 8;

	if (buffer.size() < requiredLen)
		buffer.resize(requiredLen, 0);

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

	offset += size;
}

/// Writes the given bits into the buffer, one byte at a time (MSB first)
static void writeVariableChunks(std::vector<uint8_t>& buffer, std::size_t& offset, const uint64_t value, const std::size_t size)
{
	const std::size_t fullBytes = size / 8;
	const std::size_t remainder = size % 8;

	// Write each full byte (MSB first)
	for (std::size_t i = fullBytes; i-- > 0;)
	{
		const auto chunk = static_cast<uint8_t>((value >> (i * 8 + remainder)) & 0xFF);
		writeVariableU8(buffer, offset, chunk, 8);
	}

	// Write remaining bits
	if (remainder > 0)
	{
		const auto chunk = static_cast<uint8_t>(value & (1u << remainder) - 1);
		writeVariableU8(buffer, offset, chunk, remainder);
	}
}

void writeVariableU16(std::vector<uint8_t>& buffer, std::size_t& offset, const uint16_t value, const std::size_t size)
{
	if (size > 16)
		throw std::invalid_argument("Size cannot exceed 16 bits");

	writeVariableChunks(buffer, offset, value, size);
}

void writeVariableU32(std::vector<uint8_t>& buffer, std::size_t& offset, const uint32_t value, const std::size_t size)
{
	if (size > 32)
		throw std::invalid_argument("Size cannot exceed 32 bits");

	writeVariableChunks(buffer, offset, value, size);
}

void writeVariableU64(std::vector<uint8_t>& buffer, std::size_t& offset, const uint64_t value, const std::size_t size)
{
	if (size > 64)
		throw std::invalid_argument("Size cannot exceed 64 bits");

	writeVariableChunks(buffer, offset, value, size);
}
