#include <cstring>
#include <iostream>
#include <sstream>
#include <unordered_map>

#if defined(_WIN32)
#include <winsock2.h>
#pragma comment(lib, "ws2_32.lib")
using SOCKET_LENGTH = int;
#else
#include <unistd.h>
#include <netinet/in.h>
#include <sys/socket.h>
using SOCKET = int;
using SOCKET_LENGTH = socklen_t;
#define INVALID_SOCKET (-1)
#endif

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
#if defined(_WIN32)
	WSADATA wsaData;

	if (WSAStartup(MAKEWORD(2, 2), &wsaData) != 0)
	{
		perror("WSAStartup failed.");
		return 1;
	}
#endif

	// Create socket
	const SOCKET socket = ::socket(
		// IPv4 Internet protocols
		AF_INET,

		// Supports datagrams
		SOCK_DGRAM,

		// UDP datagram sockets
		IPPROTO_UDP
	);

	if (socket == INVALID_SOCKET)
	{
		perror("Failed to create a socket.");
		return 1;
	}

	sockaddr_in server_address{};
	server_address.sin_family = AF_INET;
	server_address.sin_port = htons(server_port);
	server_address.sin_addr.s_addr = htonl(INADDR_ANY);

	// Bind socket to address
	const auto bind_error_code = bind(
		socket,
		reinterpret_cast<struct sockaddr*>(&server_address),
		sizeof(server_address)
	);

	if (bind_error_code != 0)
	{
		perror("Failed to bind the socket to an address.");
		return 1;
	}

	char buffer[1024];
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

		if (received_bytes_count < sizeof(buffer))
			buffer[received_bytes_count] = '\0';

		uint32_t identifier = (static_cast<uint64_t>(client_address.sin_addr.s_addr) << 16) | client_address.sin_port;
		std::string message;

		// Register new client
		// ReSharper disable once CppUseAssociativeContains
		if (clients.find(identifier) == clients.end())
		{
			const auto client_info = ClientInfo{.username = buffer, .address = client_address};
			clients[identifier] = client_info;

			message = "User '" + client_info.username + "' has joined the room.";
		}
		else
		{
			auto [username, _] = clients.at(identifier);

			// Unregister client
			if (strcmp(buffer, "/quit") == 0 || strcmp(buffer, "/exit") == 0)
			{
				message = "User '" + username + "' has left the room.";
				clients.erase(identifier);
			}
			// Append username
			else
				message = "[" + username + "]: " + buffer;
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
#if defined(_WIN32)
	closesocket(socket);
	WSACleanup();
#else
	close(socket);
#endif

	return 0;
}
