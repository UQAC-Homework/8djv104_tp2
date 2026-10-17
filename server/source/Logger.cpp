#include "../include/Logger.h"

Logger::Logger(std::ostream& output, const bool verbose) : out(output), verbose(verbose)
{
}

Logger::~Logger() = default;

void Logger::log_debug(const char* message) const
{
	if (!this->verbose)
		return;

	this->out << "[DEBUG] " << message << std::endl;
}

void Logger::log_debug(const std::string& message) const
{
	this->log_debug(message.c_str());
}

void Logger::log_info(const char* message) const
{
	this->out << "[INFO ] " << message << std::endl;
}

void Logger::log_info(const std::string& message) const
{
	this->log_info(message.c_str());
}

void Logger::log_error(const char* message) const
{
	this->out << "[ERROR] " << message << std::endl;
}

void Logger::log_error(const std::string& message) const
{
	this->log_error(message.c_str());
}
