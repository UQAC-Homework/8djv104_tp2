#include <iostream>
#include <sstream>
#include <unordered_map>

#include "include/Logger.h"
#include "include/PacketType.h"
#include "include/socket.h"
#include "include/serialization/PacketReader.h"

int main()
{
	// Set up
	initialize_socket();

	// Create socket
	const SOCKET socket = create_socket(9999);

	if (socket == INVALID_SOCKET)
	{
		perror("Failed to create a socket.");
		return 1;
	}

	const auto logger = Logger{std::cerr};

	unsigned char buffer[1024];

	while (true)
	{
		// Read incoming message
		sockaddr_in client_address{};
		SOCKET_LENGTH client_address_length = sizeof(client_address);

		const auto received_bytes_count = recvfrom(
			socket,
			buffer,
			sizeof(buffer),
			0,
			reinterpret_cast<sockaddr*>(&client_address),
			&client_address_length
		);

		if (received_bytes_count == -1)
		{
			perror("Failed to read from socket.");
			return 1;
		}

		if (received_bytes_count < 1)
		{
			logger.log_error("Failed to read the packet type.");
			continue;
		}

		auto reader = PacketReader{buffer, received_bytes_count};

		const auto packet_type = static_cast<PacketType>(reader.readByte());

		if (packet_type == UNDEFINED)
		{
			logger.log_error("Received an undefined packet type.");
			continue;
		}

		if (packet_type == CONNECTION_REQUEST)
		{
			const auto username_length = reader.readNybble() + 1;
			const auto username = reader.readString(username_length);

			continue;
		}

		logger.log_error("Received a packet with an unsupported packet type.");
	}

	// Close socket
	close_socket(socket);

	return 0;
}
