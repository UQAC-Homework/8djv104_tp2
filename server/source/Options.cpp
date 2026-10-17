#include "../include/Options.h"

#include <charconv>
#include <stdexcept>
#include <string>
#include <string_view>

namespace
{
	uint16_t parse_port(const std::string_view text)
	{
		unsigned value = 0;
		const char* const end = text.data() + text.size();
		const auto [stop, error] = std::from_chars(text.data(), end, value);

		if (error != std::errc{} || stop != end || value < 1 || value > 65535)
			throw std::invalid_argument("Invalid port '" + std::string(text) + "' (expected 1-65535)");

		return static_cast<uint16_t>(value);
	}

	double parse_loss(const std::string_view text)
	{
		const std::string copy(text);
		std::size_t used = 0;
		double value = 0.0;

		try
		{
			value = std::stod(copy, &used);
		}
		catch (const std::exception&)
		{
			used = 0;
		}

		// The negated comparison also rejects NaN.
		if (used == 0 || used != copy.size() || !(value >= 0.0 && value <= 1.0))
			throw std::invalid_argument("Invalid loss '" + copy + "' (expected a probability from 0 to 1)");

		return value;
	}
}

Options parse_options(const int argc, char** argv)
{
	Options options;
	bool port_given = false;

	for (int i = 1; i < argc; ++i)
	{
		const std::string_view arg = argv[i];

		if (arg == "--verbose" || arg == "-v")
		{
			options.verbose = true;
		}
		else if (arg == "--loss")
		{
			if (++i >= argc)
				throw std::invalid_argument("--loss needs a value");

			options.loss = parse_loss(argv[i]);
		}
		else if (arg.starts_with("--loss="))
		{
			options.loss = parse_loss(arg.substr(std::string_view("--loss=").size()));
		}
		else if (arg.starts_with("--"))
			throw std::invalid_argument("Unknown option '" + std::string(arg) + "'");
		else
		{
			if (port_given)
				throw std::invalid_argument("Unexpected argument '" + std::string(arg) + "'");

			options.port = parse_port(arg);
			port_given = true;
		}
	}

	return options;
}
