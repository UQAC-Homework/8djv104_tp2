#ifndef SERVER_LOGGER_H
#define SERVER_LOGGER_H
#include <ostream>

/**
 * Class responsible to handle the logging of messages
 */
class Logger
{
private:
	std::ostream& out;
	bool verbose;

public:
	explicit Logger(std::ostream& output, bool verbose);
	~Logger();

	/**
	 * Logs the given message as an error
	 */
	void log_debug(const char* message) const;
	
	/**
	 * Logs the given message as an error
	 */
	void log_debug(const std::string& message) const;

	/**
	 * Logs the given message as an error
	 */
	void log_info(const char* message) const;

	/**
	 * Logs the given message as an error
	 */
	void log_info(const std::string& message) const;

	/**
	 * Logs the given message as an error
	 */
	void log_error(const char* message) const;
	
	/**
 * Logs the given message as an error
 */
	void log_error(const std::string& message) const;
};


#endif //SERVER_LOGGER_H
