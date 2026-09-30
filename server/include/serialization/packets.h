#ifndef SERVER_PACKETS_H
#define SERVER_PACKETS_H
#include <string>

#include "../PacketType.h"

inline std::string unpack_connection(const char* buffer)
{
	int index = sizeof(PacketType) / sizeof(char);

	const unsigned char length = (buffer[index] >> 4) + 1;
	char username[length];

	for (int i = 0; i < length; i++)
	{
		const unsigned char left = buffer[index];
		const unsigned char right = buffer[index + 1];

		const char c = static_cast<char>((left & 0b1111) << 4 | (right >> 4));
		username[i] = c;
		index++;
	}

	return username;
}

#endif //SERVER_PACKETS_H
