#include "../../../include/network/serialization/PacketWriter.h"

#include <cmath>
#include <stdexcept>
#include <vector>

#include "../../../include/network/serialization/write_helper.h"

std::vector<uint8_t> PacketWriter::getBuffer() const
{
	return buffer;
}

bool PacketWriter::canWrite(const std::size_t size) const
{
	return offset + size <= MAXIMUM_PACKET_SIZE * 8;
}

void PacketWriter::resizeFor(const std::size_t size)
{
	const auto current_length = buffer.size() * 8;
	const auto target_length = offset + size;

	if (current_length >= target_length)
		return;

	std::size_t target_size = target_length / 8;

	if (target_length % 8 != 0)
		target_size++;

	buffer.resize(target_size, 0);
}

void PacketWriter::writeNybble(const uint8_t value)
{
	if (!canWrite(4))
		throw std::overflow_error("Cannot write a nybble to the buffer.");

	resizeFor(4);

	offset = writeVariableU8(
		buffer.data(),
		offset,
		value & 0b1111,
		4
	);
}

void PacketWriter::writeSextet(const uint8_t value)
{
	if (!canWrite(6))
		throw std::overflow_error("Cannot write a sextet to the buffer.");

	resizeFor(6);

	offset = writeVariableU8(
		buffer.data(),
		offset,
		value & 0b111111,
		6
	);
}

void PacketWriter::writeByte(const uint8_t value)
{
	if (!canWrite(8))
		throw std::overflow_error("Cannot write a byte to the buffer.");

	resizeFor(8);

	offset = writeVariableU8(
		buffer.data(),
		offset,
		value,
		8
	);
}

void PacketWriter::writeDeclet(const uint16_t value)
{
	if (!canWrite(10))
		throw std::overflow_error("Cannot write a declet to the buffer.");

	resizeFor(10);

	offset = writeVariableU16(
		buffer.data(),
		offset,
		value,
		10
	);
}

void PacketWriter::writeQuadword(const uint64_t value)
{
	if (!canWrite(64))
		throw std::overflow_error("Cannot write a quadword to the buffer.");

	resizeFor(64);

	offset = writeVariableU64(
		buffer.data(),
		offset,
		value,
		64
	);
}

void PacketWriter::writeString(const std::string& s)
{
	const auto valueLength = s.size();
	const auto bit_count = valueLength * 8;

	if (!canWrite(bit_count))
		throw std::overflow_error(
			"Cannot write a string of " + std::to_string(valueLength) +
			" characters to the buffer.");

	resizeFor(bit_count);

	for (const char c : s)
		writeByte(static_cast<uint8_t>(c));
}
