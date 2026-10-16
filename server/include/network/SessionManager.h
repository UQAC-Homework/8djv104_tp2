#ifndef SERVER_SESSIONMANAGER_H
#define SERVER_SESSIONMANAGER_H
#include <chrono>
#include <optional>
#include <ranges>
#include <string>
#include <unordered_map>
#include <netinet/in.h>

constexpr std::size_t HISTORY_SIZE = 64;

/// User identifier reserved for the server
constexpr unsigned SERVER_USER_IDENTIFIER = 0b000000;

struct StoredMessage
{
	bool valid = false;
	uint8_t id = 0;
	std::string username;
	std::string text;
};

struct Session
{
	sockaddr_in address{};
	uint8_t clientId = 0;
	uint8_t userId = 0;
	std::string username;

	// Receive tracking for messages submitted by this client.
	uint8_t lastClientMessageId = 0xFF;
	uint64_t receivedClientMessages = UINT64_MAX;

	// Server-to-client message sequence and retransmission history.
	uint8_t nextServerMessageId = 0;
	std::array<StoredMessage, HISTORY_SIZE> history{};

	std::chrono::steady_clock::time_point lastPacket = std::chrono::steady_clock::now();
	bool suspicious = false;
	std::chrono::steady_clock::time_point nextHeartbeatRequest = std::chrono::steady_clock::now();

	/// Refresh the session's activity
	void refresh();
};

/// Class responsible to manage user sessions
class SessionManager
{
public:
	/// Attempts to find the session opened for the given client 
	[[nodiscard]] Session* find_session(const sockaddr_in& address, uint8_t client_identifier);

	/// Attempts to create a session for the given client
	[[nodiscard]] std::optional<uint8_t> create_session(
		const sockaddr_in& address,
		uint8_t client_identifier,
		const std::string& username
	);

	/// Deletes the session of the given user
	void delete_session(uint8_t user_identifier);

	/// Gets the session of the given user
	[[nodiscard]] Session* get_session(uint8_t user_identifier);

	template <typename F>
	void for_each_session(F&& fn);

private:
	std::unordered_map<uint8_t, Session> sessions;

	/// Checks if the given addresses are the same
	static bool are_same_address(const sockaddr_in& a, const sockaddr_in& b);

	/// Finds an unused user identifier
	[[nodiscard]] uint8_t allocate_user_identifier() const;
};

template <typename F>
void SessionManager::for_each_session(F&& fn)
{
	for (auto& session : sessions | std::views::values)
		fn(session);
}


#endif //SERVER_SESSIONMANAGER_H
