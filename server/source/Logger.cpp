#include "../include/Logger.h"

Logger::Logger(std::ostream& error_output)
	: error_out(error_output)
{
}

Logger::~Logger() = default;

void Logger::log_error(const char* message) const
{
	this->error_out << message << std::endl;
}
