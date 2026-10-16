#include "../../include/serialization/PackerWriter.h"

#include <stdexcept>
#include <vector>
#include "../../include/serialization/WriteHelper.h"

std::vector<uint8_t> PacketWriter::getBuffer() const
{
	return buffer;
}

bool PacketWriter::canWrite(const std::size_t size) const
{
	return offset + size <= MAXIMUM_PACKET_SIZE * 8;
}

void PacketWriter::writeNybble(const uint8_t value)
{
	if (!canWrite(4))
		throw std::overflow_error("Cannot write a nybble to the buffer.");

	writeVariableU8(buffer, offset, value & 0b1111, 4);
}

void PacketWriter::writeSextet(const uint8_t value)
{
	if (!canWrite(6))
		throw std::overflow_error("Cannot write a sextet to the buffer.");

	writeVariableU8(buffer, offset, value & 0b111111, 6);
}

void PacketWriter::writeByte(const uint8_t value)
{
	if (!canWrite(8))
		throw std::overflow_error("Cannot write a byte to the buffer.");

	writeVariableU8(buffer, offset, value, 8);
}

void PacketWriter::writeDeclet(const uint16_t value)
{
	if (!canWrite(10))
		throw std::overflow_error("Cannot write a declet to the buffer.");

	writeVariableU16(buffer, offset, value, 10);
}

void PacketWriter::writeQuadword(const uint64_t value)
{
	if (!canWrite(64))
		throw std::overflow_error("Cannot write a quadword to the buffer.");

	writeVariableU64(buffer, offset, value, 64);
}

void PacketWriter::writeString(const std::string& s)
{
	const std::size_t valueLength = s.size();

	if (!canWrite(valueLength * 8))
		throw std::overflow_error(
			"Cannot write a string of " + std::to_string(valueLength) +
			" characters to the buffer.");

	for (const char c : s)
		writeByte(static_cast<uint8_t>(c));
}
