#include <iostream>
#include <optional>

#include "include/Logger.h"
#include "include/Options.h"
#include "include/network/Server.h"

int main(const int argc, char** argv)
{
	Options options;

	try
	{
		options = parse_options(argc, argv);
	}
	catch (const std::invalid_argument& e)
	{
		std::cerr << "Error: " << e.what() << "\n\n" << USAGE << "\n";
		return 2;
	}

	Logger logger(std::cout, options.verbose);
	LossSimulator lossSimulator(options.loss);

	try
	{
		Server server(options.port, logger, lossSimulator);
		server.run();
	}
	catch (const std::exception& e)
	{
		std::cerr << "Server error: " << e.what() << "\n";
		return 1;
	}
}
