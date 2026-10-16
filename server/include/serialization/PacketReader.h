#ifndef SERVER_PACKETREADER_H
#define SERVER_PACKETREADER_H

#include <cstddef>
#include <cstdint>
#include <string>

/// Structure used to read packets from a binary format
class PacketReader
{
public:
	/**
	 * Creates a new reader
	 * @param buffer Buffer to read
	 * @param length Length of the buffer in bytes
	 */
	PacketReader(const uint8_t* buffer, std::size_t length);

	/// Reads a nybble (4 bits)
	uint8_t readNybble();

	/// Reads 6 bits
	uint8_t readSextet();

	/// Reads a byte
	uint8_t readByte();

	/// Reads 10 bits
	uint16_t readDeclet();

	/// Reads a quadword
	uint64_t readQuadword();

	/// Reads a string of the given character length
	std::string readString(std::size_t length);

private:
	/// Buffer to read from
	const uint8_t* buffer;

	/// Length of the buffer in bits
	std::size_t length;

	/// Offset from the start of the buffer in bits
	std::size_t offset = 0;

	/// Checks if the reader can read the given amount of bits
	[[nodiscard]] bool canRead(std::size_t size) const;
};

#endif //SERVER_PACKETREADER_H
