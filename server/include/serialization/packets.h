#ifndef SERVER_PACKETS_H
#define SERVER_PACKETS_H
#include <string>

/**
 * Unpacks the given data as a @ref PacketType::Connection from the given offset
 * 
 * @param buffer Raw packet bytes
 * @param offset Position of the first data byte
 * 
 * @return The decoded username
 */
inline std::string unpack_connection(const char* buffer, int offset)
{
	const unsigned char length = (buffer[offset] >> 4) + 1;
	char username[length];

	for (int i = 0; i < length; i++)
	{
		const unsigned char left = buffer[offset];
		const unsigned char right = buffer[offset + 1];

		const char c = static_cast<char>((left & 0b1111) << 4 | (right >> 4));
		username[i] = c;
		offset++;
	}

	return username;
}

#endif //SERVER_PACKETS_H
