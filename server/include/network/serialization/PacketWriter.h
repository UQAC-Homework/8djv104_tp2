#ifndef SERVER_PACKERWRITER_H
#define SERVER_PACKERWRITER_H

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>
#include <bits/stdint-uintn.h>

/// Structure used to write packets into a binary format
class PacketWriter
{
public:
	/// Maximum size of a packet in bytes
	static constexpr std::size_t MAXIMUM_PACKET_SIZE = 1024;

	/// Creates a new writer
	PacketWriter() = default;

	/// Gets the buffer of the writer
	[[nodiscard]] std::vector<uint8_t> getBuffer() const;

	/// Writes the given nybble (4 bits)
	void writeNybble(uint8_t value);

	/// Writes the given 6 bits
	void writeSextet(uint8_t value);

	/// Writes the given byte
	void writeByte(uint8_t value);

	/// Writes the given 10 bits
	void writeDeclet(uint16_t value);

	/// Writes the given quadword
	void writeQuadword(uint64_t value);

	/// Writes the given string
	void writeString(const std::string& s);

private:
	/// Buffer to write to
	std::vector<uint8_t> buffer;

	/// Offset from the start of the buffer in bits
	std::size_t offset = 0;

	/// Checks if the writer can write the given amount of bits
	[[nodiscard]] bool canWrite(std::size_t size) const;

	/// Resizes the buffer to be able to write the given amount of bits
	void resizeFor(std::size_t size);
};


#endif //SERVER_PACKERWRITER_H
