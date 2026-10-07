#include "../../include/serialization/PacketReader.h"

#include <stdexcept>

bool PacketReader::canRead(const size_t size) const
{
	return this->offset + size <= this->length;
}

PacketReader::PacketReader(const unsigned char* buffer, const ssize_t length) : length(length * 8), offset(0)
{
	this->buffer = buffer;
}

unsigned char PacketReader::readNybble()
{
	if (!this->canRead(4))
		throw std::runtime_error("Cannot read a nybble from the buffer.");

	unsigned char value;

	const auto remaining_bits = this->offset % 4;
	const auto is_upper = this->offset % 8 == 0;
	const auto position = this->offset / 8;

	// If on two bytes, merge
	if (remaining_bits > 0)
	{
		const auto bits_to_read = 4 - remaining_bits;

		value = (this->buffer[position] << bits_to_read) | (this->buffer[position + 1] >> bits_to_read);
	}
	// If on upper of byte, shift
	else if (is_upper)
		value = this->buffer[position] >> 4;
		// If on lower of byte, read
	else
		value = this->buffer[position];

	this->offset += 4;

	// Truncate the output
	value &= 0b1111;

	return value;
}

unsigned char PacketReader::readByte()
{
	if (!this->canRead(8))
		throw std::runtime_error("Cannot read a byte from the buffer.");

	unsigned char value;

	const auto remaining_bits = this->offset % 8;
	const auto position = this->offset / 8;

	// If on two bytes, merge
	if (remaining_bits > 0)
	{
		const auto bits_to_read = 8 - remaining_bits;

		value = (this->buffer[position] << bits_to_read) | (this->buffer[position + 1] >> bits_to_read);
	}
	// If on whole byte, read
	else
		value = this->buffer[position];

	this->offset += 8;

	return value;
}

char* PacketReader::readString(const ssize_t size)
{
	if (!this->canRead(size * 8))
	{
		throw std::runtime_error(
			"Cannot read a string of " + std::to_string(size) + " characters from the buffer."
		);
	}

	char data[size + 1];

	for (int i = 0; i < size; i++)
		data[i] = static_cast<char>(this->readByte());

	data[size] = '\0';

	return data;
}
