#include <algorithm>
#include <cmath>
#include <iostream>
#include <optional>

#include "include/network/Server.h"
#include "include/network/SessionManager.h"

int main(const int argc, char** argv)
{
	uint16_t port = 4000;

	if (argc > 2)
	{
		std::cerr << "Usage: " << argv[0] << " [port]\n";
		return 1;
	}

	if (argc == 2)
	{
		char* end = nullptr;
		const long value = std::strtol(argv[1], &end, 10);

		if (!end || *end != '\0' || value < 1 || value > 65535)
		{
			std::cerr << "Invalid port\n";
			return 1;
		}

		port = static_cast<uint16_t>(value);
	}

	try
	{
		Server server(port);
		server.run();
	}
	catch (const std::exception& e)
	{
		std::cerr << "Server error: " << e.what() << "\n";
		return 1;
	}
}
