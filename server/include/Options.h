#ifndef SERVER_OPTIONS_H
#define SERVER_OPTIONS_H

#include <cstdint>

/// Command line: [port] [--loss P] [--verbose]
struct Options
{
	uint16_t port = 9999;
	double loss = 0.0; // probability in [0, 1] that a datagram is dropped
	bool verbose = false;
};

constexpr auto USAGE =
	"Usage: server [port] [--loss P] [--verbose]\n"
	"\n"
	"  port       UDP port to listen on, 1-65535 (default 9999)\n"
	"  --loss P   drop each datagram with probability P, 0 to 1 (default 0)\n"
	"  --verbose  log every packet";

Options parse_options(int argc, char** argv);

#endif //SERVER_OPTIONS_H
