#ifndef SERVER_LOGGER_H
#define SERVER_LOGGER_H
#include <ostream>

/**
 * Class responsible to handle the logging of messages
 */
class Logger
{
private:
	std::ostream& error_out;

public:
	explicit Logger(std::ostream& error_output);
	~Logger();

	/**
	 * Logs the given message as an error
	 */
	void log_error(const char* message) const;
};


#endif //SERVER_LOGGER_H
