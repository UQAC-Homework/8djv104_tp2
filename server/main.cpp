#include <cstring>
#include <iostream>
#include <sstream>
#include <unordered_map>

#include "include/Logger.h"
#include "include/PacketType.h"
#include "include/socket.h"
#include "include/serialization/PacketReader.h"
#include "include/serialization/packets.h"

namespace
{
	struct ClientInfo
	{
		std::string username;
		sockaddr_in address{};
	};
}

int main(const int argc, char* argv[])
{
	// Parse arguments
	size_t server_port = 9999;

	if (argc > 1)
		server_port = strtol(argv[1], nullptr, 10);

	// Set up
	initialize_socket();

	// Create socket
	const SOCKET socket = create_socket(server_port);

	if (socket == INVALID_SOCKET)
	{
		perror("Failed to create a socket.");
		return 1;
	}

	const auto logger = Logger{std::cerr};

	unsigned char buffer[1024];
	std::unordered_map<uint32_t, ClientInfo> clients;

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

			//const auto username = unpack_connection(buffer, 1);

			std::string message = "User '" + username + "' has joined the room.";

			// Send message to client
			const auto _ = sendto(
				socket,
				message.c_str(),
				message.length(),
				0,
				reinterpret_cast<const sockaddr*>(&client_address),
				sizeof(client_address)
			);
			continue;
		}

		logger.log_error("Received a packet with an unsupported packet type.");

		if (received_bytes_count < sizeof(buffer))
			buffer[received_bytes_count] = '\0';

		uint32_t identifier = (static_cast<uint64_t>(client_address.sin_addr.s_addr) << 16) | client_address.sin_port;
		std::string message;

		// Register new client
		// ReSharper disable once CppUseAssociativeContains
		if (clients.find(identifier) == clients.end())
		{
			const auto client_info = ClientInfo{
				.username = reinterpret_cast<const char*>(buffer), .address = client_address
			};
			clients[identifier] = client_info;

			message = "User '" + client_info.username + "' has joined the room.";
		}
		else
		{
			auto [username, _] = clients.at(identifier);

			// Unregister client
			if (strcmp(reinterpret_cast<const char*>(buffer), "/quit") == 0 || strcmp(
				reinterpret_cast<const char*>(buffer), "/exit") == 0)
			{
				message = "User '" + username + "' has left the room.";
				clients.erase(identifier);
			}
			// Append username
			else
				message = "[" + username + "]: " + reinterpret_cast<const char*>(buffer);
		}

		for (const auto& [client_id, client_info] : clients)
		{
			if (client_id == identifier)
				continue;

			// Send message to client
			const auto _ = sendto(
				socket,
				message.c_str(),
				message.length(),
				0,
				reinterpret_cast<const sockaddr*>(&client_info.address),
				sizeof(client_info.address)
			);
		}
	}

	// Close socket
	close_socket(socket);

	return 0;
}
