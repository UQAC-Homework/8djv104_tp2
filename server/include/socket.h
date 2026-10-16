// File defining the abstractions of the socket

#ifndef SERVER_SOCKET_H
#define SERVER_SOCKET_H

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

/**
 * Initializes the required framework for sockets
 */
inline void initialize_socket()
{
#if defined(_WIN32)
	WSADATA wsaData;

	if (WSAStartup(MAKEWORD(2, 2), &wsaData) != 0)
	{
		perror("WSAStartup failed.");
		exit(1);
	}
#endif
}

/**
 * Creates a socket at the given port
 */
inline SOCKET create_socket(const size_t port)
{
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
		return INVALID_SOCKET;
	}

	sockaddr_in address{};
	address.sin_family = AF_INET;
	address.sin_port = htons(port);
	address.sin_addr.s_addr = htonl(INADDR_ANY);

	// Bind socket to address
	const auto bind_error_code = bind(
		socket,
		reinterpret_cast<struct sockaddr*>(&address),
		sizeof(address)
	);

	if (bind_error_code != 0)
	{
		perror("Failed to bind the socket to an address.");
		return INVALID_SOCKET;
	}

	return socket;
}

/**
 * Closes the given socket
 */
inline void close_socket(SOCKET socket)
{
#if defined(_WIN32)
	closesocket(socket);
	WSACleanup();
#else
	close(socket);
#endif
}

#endif //SERVER_SOCKET_H
