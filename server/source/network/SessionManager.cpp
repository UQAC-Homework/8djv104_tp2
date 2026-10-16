#include "../../include/network/SessionManager.h"

#include <memory>
#include <ranges>

void Session::refresh()
{
	lastPacket = std::chrono::steady_clock::now();
	suspicious = false;
}

uint8_t Session::add_message(const std::string& sender_name, const std::string& message)
{
	StoredMessage stored;
	stored.valid = true;
	stored.id = nextServerMessageId;
	stored.username = sender_name;
	stored.text = message;

	history[stored.id % HISTORY_SIZE] = stored;
	nextServerMessageId = static_cast<uint8_t>(nextServerMessageId + 1);

	return stored.id;
}

Session* SessionManager::find_session(const sockaddr_in& address, const uint8_t client_identifier)
{
	for (auto& session : sessions | std::views::values)
	{
		if (are_same_address(session.address, address) && session.clientId == client_identifier)
			return &session;
	}

	return nullptr;
}


std::optional<uint8_t> SessionManager::create_session(const sockaddr_in& address, const uint8_t client_identifier,
                                                      const std::string& username)
{
	const uint8_t id = allocate_user_identifier();

	if (id == SERVER_USER_IDENTIFIER)
		return std::nullopt;

	Session s;
	s.address = address;
	s.clientId = client_identifier;
	s.userId = id;
	s.username = username;

	sessions.emplace(id, std::move(s));

	return id;
}

void SessionManager::delete_session(const uint8_t user_identifier)
{
	if (user_identifier == SERVER_USER_IDENTIFIER)
		return;

	sessions.erase(user_identifier);
}

Session* SessionManager::get_session(const uint8_t user_identifier)
{
	const auto it = sessions.find(user_identifier);

	if (it == sessions.end())
		return nullptr;

	return &it->second;
}

bool SessionManager::are_same_address(const sockaddr_in& a, const sockaddr_in& b)
{
	return a.sin_family == b.sin_family && a.sin_port == b.sin_port &&
		a.sin_addr.s_addr == b.sin_addr.s_addr;
}

uint8_t SessionManager::allocate_user_identifier() const
{
	for (auto id = 1; id <= 63; ++id)
	{
		if (sessions.contains(id))
			continue;

		return id;
	}

	return SERVER_USER_IDENTIFIER;
}
