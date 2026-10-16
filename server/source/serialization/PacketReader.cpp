#include "../../include/serialization/PacketReader.h"
#include "../../include/serialization/ReadHelper.h"

#include <stdexcept>

PacketReader::PacketReader(const uint8_t* buffer, const std::size_t length)
	: buffer(buffer), length(length * 8)
{
}

bool PacketReader::canRead(const std::size_t size) const
{
	return offset + size <= length;
}

uint8_t PacketReader::readNybble()
{
	if (!canRead(4))
		throw std::out_of_range("Cannot read a nybble from the buffer.");

	return readVariableU8(buffer, offset, 4) & 0b1111;
}

uint8_t PacketReader::readSextet()
{
	if (!canRead(6))
		throw std::out_of_range("Cannot read a sextet from the buffer.");

	return readVariableU8(buffer, offset, 6) & 0b111111;
}

uint8_t PacketReader::readByte()
{
	if (!canRead(8))
		throw std::out_of_range("Cannot read a byte from the buffer.");

	return readVariableU8(buffer, offset, 8);
}

uint16_t PacketReader::readDeclet()
{
	if (!canRead(10))
		throw std::out_of_range("Cannot read a declet from the buffer.");

	return readVariableU16(buffer, offset, 10);
}

uint64_t PacketReader::readQuadword()
{
	if (!canRead(64))
		throw std::out_of_range("Cannot read a quadword from the buffer.");

	return readVariableU64(buffer, offset, 64);
}

std::string PacketReader::readString(const std::size_t length)
{
	// Check the length in bytes first so `length * 8` can't overflow
	if (length > (this->length - offset) / 8)
		throw std::out_of_range(
			"Cannot read a string of " + std::to_string(length) +
			" characters from the buffer.");

	std::string result;
	result.reserve(length);

	for (std::size_t i = 0; i < length; ++i)
		result.push_back(static_cast<char>(readByte()));

	return result;
}
