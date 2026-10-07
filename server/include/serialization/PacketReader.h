#ifndef SERVER_PACKETREADER_H
#define SERVER_PACKETREADER_H
#include <sys/types.h>

/// Structure used to read packets from a binary format
struct PacketReader
{
private:
	/// Buffer to read from
	const unsigned char* buffer;

	/// Length of the buffer in bits
	const ssize_t length;

	/// Offset from the start of the buffer in bits
	ssize_t offset;

	/// Checks if the reader can read the given amount of bits
	[[nodiscard]] bool canRead(size_t size) const;

public:
	/**
	 * Creates a new reader
	 * @param buffer Buffer to read
	 * @param length Length of the buffer in bytes
	 */
	explicit PacketReader(const unsigned char* buffer, ssize_t length);

	/// Reads a nybble
	unsigned char readNybble();

	/// Reads a byte
	unsigned char readByte();
	
	/// Reads a string of the given size
	char* readString(ssize_t size);
};

#endif //SERVER_PACKETREADER_H
