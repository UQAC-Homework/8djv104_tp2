#include "../../include/network/Server.h"

#include <algorithm>
#include <array>
#include <cerrno>
#include <chrono>
#include <cstdio>
#include <iostream>
#include <optional>
#include <string>
#include <vector>

#include "../../include/socket.h"
#include "../../include/network/packets/types.h"
#include "../../include/network/serialization/PacketReader.h"
#include "../../include/network/serialization/PacketWriter.h"

using namespace std::chrono_literals;

namespace
{
	using Clock = std::chrono::steady_clock;

	// ---- Protocol limits -------------------------------------------------

	constexpr std::size_t MAX_DATAGRAM = 1024;
	constexpr std::size_t MAX_USERNAME = 16;
	constexpr std::size_t MAX_MESSAGE = 990;

	// ---- Timing ----------------------------------------------------------

	constexpr int POLL_TIMEOUT_US = 50'000;
	constexpr auto INACTIVITY_WARNING = 1500ms;
	constexpr auto INACTIVITY_TIMEOUT = 3s;
	constexpr auto HEARTBEAT_REQUEST_INTERVAL = 200ms;

	// ---- Message identifiers (8 bits, wrapping) ----------------------------

	/// Number of steps to go from `from` to `to`, modulo 256.
	uint8_t forward_distance(const uint8_t from, const uint8_t to)
	{
		return static_cast<uint8_t>(to - from);
	}

	bool within(const uint8_t value, const uint8_t low, const uint8_t high)
	{
		return value >= low && value <= high;
	}

	/// The `Last Message Identifier` + `Received Messages` pair that most
	/// packets carry. It describes what the *sender* received from us.
	struct PeerAck
	{
		uint8_t last;
		uint64_t received;
	};

	PeerAck read_peer_ack(PacketReader& reader)
	{
		const auto last = reader.readByte();
		const auto received = reader.readQuadword();

		return {.last = last, .received = received};
	}

	/// Did the peer report this message of ours as not received?
	/// Bit n of `received` describes the message `last - 1 - n`.
	bool is_missing_for_peer(const uint8_t id, const PeerAck& ack)
	{
		// Sent after the last message the peer saw.
		if (within(forward_distance(ack.last, id), 1, 127))
			return true;

		// Older than `last`: the peer's bitmap says whether it arrived.
		const auto behind = forward_distance(id, ack.last);

		if (!within(behind, 1, 64))
			return false;

		return ((ack.received >> (behind - 1)) & 1u) == 0;
	}

	/// Classifies a message from the client and updates what we received.
	Arrival classify_client_message(Session& s, const uint8_t id)
	{
		const auto ahead = forward_distance(s.lastClientMessageId, id);

		if (within(ahead, 1, 127))
		{
			// Moving `last` forward shifts the old bits up, marks the old
			// `last` as received, and leaves everything in between as 0.
			s.receivedClientMessages = ahead >= 64 ? 0 : s.receivedClientMessages << ahead;

			if (ahead <= 64)
				s.receivedClientMessages |= uint64_t{1} << (ahead - 1);

			s.lastClientMessageId = id;
			return {.kind = ArrivalKind::New, .distance = ahead};
		}

		const auto behind = forward_distance(id, s.lastClientMessageId);

		// 0 is a duplicate of `last`, above 64 is older than the window.
		if (behind == 0 || behind > 64)
			return {.kind = ArrivalKind::Ignored, .distance = 0};

		const uint64_t bit = uint64_t{1} << (behind - 1);

		if (s.receivedClientMessages & bit)
			return {.kind = ArrivalKind::Ignored, .distance = 0};

		s.receivedClientMessages |= bit;
		return {.kind = ArrivalKind::Late, .distance = 0};
	}

	// ---- Socket helpers ----------------------------------------------------

	struct Datagram
	{
		std::array<uint8_t, MAX_DATAGRAM> data{};
		std::size_t size = 0;
		sockaddr_in peer{};
	};

	/// Waits for incoming data, but not longer than `POLL_TIMEOUT_US`.
	bool wait_for_readable(const auto sock)
	{
		fd_set set;
		FD_ZERO(&set);
		FD_SET(sock, &set);
		timeval timeout{.tv_sec = 0, .tv_usec = POLL_TIMEOUT_US};

		const int ready = select(sock + 1, &set, nullptr, nullptr, &timeout);

		if (ready < 0 && errno != EINTR)
			std::perror("select");

		return ready > 0;
	}

	std::optional<Datagram> receive_datagram(const auto sock)
	{
		Datagram datagram;
		socklen_t peerLength = sizeof(datagram.peer);

		const ssize_t count = recvfrom(sock, datagram.data.data(), datagram.data.size(), 0,
		                               reinterpret_cast<sockaddr*>(&datagram.peer), &peerLength);

		if (count <= 0)
			return std::nullopt;

		datagram.size = static_cast<std::size_t>(count);
		return datagram;
	}
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

Server::Server(const uint16_t port, Logger& logger, LossSimulator& loss_simulator) : port(port), logger(logger), lossSimulator(loss_simulator)
{
	initialize_socket();
	socket = create_socket(port);
}

Server::~Server()
{
	close_socket(socket);
}

void Server::run()
{
	logger.log_info("Listening on UDP port " + std::to_string(port) + "\n");

	while (true)
	{
		tick();

		if (!wait_for_readable(socket))
			continue;

		const auto datagram = receive_datagram(socket);

		if (datagram)
			handle_packet(datagram->data.data(), datagram->size, datagram->peer);
	}
}

// ---------------------------------------------------------------------------
// Sending
// ---------------------------------------------------------------------------

void Server::send_packet(const sockaddr_in& to, const std::vector<uint8_t>& bytes)
{
	if (lossSimulator.should_drop())
	{
		logger.log_debug("Packet dropped.");
		return;
	}
	
	(void)sendto(socket, bytes.data(), bytes.size(), 0,
	             reinterpret_cast<const sockaddr*>(&to), sizeof(to));
}

void Server::send_type(const sockaddr_in& to, const uint8_t type)
{
	send_packet(to, std::vector{type});
}

void Server::send_user_packet(const sockaddr_in& to, const uint8_t type, const uint8_t userId)
{
	PacketWriter writer;
	writer.writeByte(type);
	writer.writeSextet(userId);

	send_packet(to, writer.getBuffer());
}

/// HEARTBEAT_RECEIVED and HEARTBEAT_REQUEST: our receive state for this client.
void Server::send_receive_status(const sockaddr_in& to, const uint8_t type, const Session& s)
{
	PacketWriter writer;
	writer.writeByte(type);
	writer.writeByte(s.lastClientMessageId);
	writer.writeQuadword(s.receivedClientMessages);

	send_packet(to, writer.getBuffer());
}

void Server::send_message_request(const Session* recipient)
{
	if (recipient == nullptr)
		return;

	PacketWriter writer;
	writer.writeByte(MESSAGE_REQUEST);
	writer.writeSextet(recipient->userId);
	writer.writeByte(recipient->lastClientMessageId);
	writer.writeQuadword(recipient->receivedClientMessages);

	send_packet(recipient->address, writer.getBuffer());
}

void Server::send_message_broadcast(
	const Session* recipient,
	const uint8_t message_identifier,
	const std::string& username,
	const std::string& message)
{
	if (recipient == nullptr)
		return;

	if (username.empty() || username.size() > MAX_USERNAME)
		return;

	if (message.empty() || message.size() > MAX_MESSAGE)
		return;

	PacketWriter writer;
	writer.writeByte(MESSAGE_BROADCAST);
	writer.writeByte(message_identifier);
	writer.writeByte(recipient->lastClientMessageId);
	writer.writeQuadword(recipient->receivedClientMessages);
	writer.writeNybble(username.size() - 1);
	writer.writeString(username);
	writer.writeDeclet(message.size());
	writer.writeString(message);

	logger.log_debug("Sending 'MESSAGE_BROADCAST' to user=" + std::to_string(recipient->userId));
	send_packet(recipient->address, writer.getBuffer());
}

/// Stores the message in the recipient's history, then sends it.
void Server::send_new_broadcast(Session* recipient, const std::string& username, const std::string& message)
{
	if (recipient == nullptr)
		return;

	const auto message_id = recipient->add_message(username, message);

	send_message_broadcast(recipient, message_id, username, message);
}

// ---------------------------------------------------------------------------
// Receiving
// ---------------------------------------------------------------------------

void Server::handle_packet(const uint8_t* data, const std::size_t size, const sockaddr_in& peer)
{
	if (size == 0)
		return;

	PacketReader reader(data, size);

	try
	{
		const auto type = reader.readByte();

		switch (type)
		{
		case CONNECTION_REQUEST:
			handle_connection_request(reader, peer);
			break;

		case CONNECTION_CLOSED:
			handle_connection_closed(reader);
			break;

		case MESSAGE_SUBMIT:
			handle_message_submit(reader);
			break;

		case MESSAGE_REQUEST:
			handle_message_request(reader);
			break;

		case HEARTBEAT_BROADCAST:
			handle_heartbeat(reader, peer);
			break;

		default:
			logger.log_debug("Received unsupported message type: " + std::to_string(type));
			break; // unsupported types are ignored
		}
	}
	catch (std::exception& e)
	{
		std::cerr << "Error while handling packet: " << e.what() << std::endl;
	}
}

void Server::handle_connection_request(PacketReader& reader, const sockaddr_in& peer)
{
	const auto client_identifier = reader.readNybble();
	const auto username_length = reader.readNybble() + 1;

	if (username_length > MAX_USERNAME)
	{
		logger.log_error("Username cannot be larger than " + std::to_string(MAX_USERNAME) + " characters.");
		return;
	}

	const std::string name = reader.readString(username_length);
	reader.assert_finish();

	// A retry must get the original user id back, even if the username differs.
	const auto existing_session = sessionManager.find_session(peer, client_identifier);

	if (existing_session != nullptr)
	{
		logger.log_debug("Refreshing session of client=" + std::to_string(existing_session->clientId));
		existing_session->refresh();
		logger.log_debug("Sending 'CONNECTION_ACCEPTED' to user=" + std::to_string(existing_session->userId));
		send_user_packet(peer, CONNECTION_ACCEPTED, existing_session->userId);
		return;
	}

	const auto user_identifier = sessionManager.create_session(peer, client_identifier, name);

	if (!user_identifier.has_value())
	{
		logger.log_debug("Sending 'CONNECTION_REJECTED' to user=" + std::to_string(user_identifier.value()));
		send_type(peer, CONNECTION_REJECTED);
		return;
	}

	logger.log_debug("Sending 'CONNECTION_ACCEPTED' to user=" + std::to_string(user_identifier.value()));
	send_user_packet(peer, CONNECTION_ACCEPTED, user_identifier.value());
}

void Server::handle_connection_closed(PacketReader& reader)
{
	const auto user_identifier = reader.readSextet();

	reader.skip(2);
	reader.assert_finish();

	// Identifier 0 is reserved for the server's own shutdown notice.
	if (user_identifier == 0)
	{
		logger.log_error("Received connection termination from server.");
		return;
	}

	if (sessionManager.get_session(user_identifier) == nullptr)
	{
		logger.log_debug(
			"Attempt to close session of user=" + std::to_string(user_identifier) + ", but it is already closed.");
		return;
	}

	logger.log_debug("Session was closed by user=" + std::to_string(user_identifier));
	sessionManager.delete_session(user_identifier);
}

void Server::handle_heartbeat(PacketReader& reader, const sockaddr_in& peer)
{
	const auto user_identifier = reader.readSextet();
	const auto [last, received] = read_peer_ack(reader);

	reader.skip(2);
	reader.assert_finish();

	const auto session = sessionManager.get_session(user_identifier);

	if (session == nullptr)
	{
		logger.log_debug("Received heartbeat of a closed session.");
		return;
	}

	session->refresh();
	process_peer_ack(session, last, received);

	logger.log_debug("Sending 'HEARTBEAT_RECEIVED' to user=" + std::to_string(user_identifier));
	send_receive_status(peer, HEARTBEAT_RECEIVED, *session);
}

void Server::handle_message_request(PacketReader& reader)
{
	const auto user_identifier = reader.readSextet();
	const auto [last, received] = read_peer_ack(reader);

	reader.skip(2);
	reader.assert_finish();

	const auto session = sessionManager.get_session(user_identifier);

	if (session == nullptr)
	{
		logger.log_debug("Received a message request from a closed session.");
		return;
	}

	session->refresh();
	process_peer_ack(session, last, received);
}

void Server::handle_message_submit(PacketReader& reader)
{
	// -- Parse ---------------------------------------------------------------
	const auto user_identifier = reader.readSextet();
	const auto [last, received] = read_peer_ack(reader);
	const auto message_identifier = reader.readByte();
	const auto message_length = reader.readDeclet();

	// Empty messages must be ignored; they would also burn a message id.
	if (message_length == 0 || message_length > MAX_MESSAGE)
	{
		logger.log_error("Message cannot be empty or be larger than " + std::to_string(MAX_MESSAGE) + " characters.");
		return;
	}

	const auto message = reader.readString(message_length);
	reader.assert_finish();

	// -- Authenticate --------------------------------------------------------
	const auto sender = sessionManager.get_session(user_identifier);

	if (sender == nullptr)
	{
		logger.log_debug("Message submitted by a closed session.");
		return;
	}

	sender->refresh();

	// -- Reliability ---------------------------------------------------------
	// Resend what the sender says it missed, even if this packet is a duplicate.
	process_peer_ack(sender, last, received);

	const auto [kind, distance] = classify_client_message(*sender, message_identifier);

	if (kind == ArrivalKind::Ignored)
		return;

	// A jump of more than one means messages in between never arrived.
	if (kind == ArrivalKind::New && distance > 1)
		send_message_request(sender);

	// -- Broadcast -----------------------------------------------------------
	// Each recipient has its own server-side counter and history.
	sessionManager.for_each_session([&](Session& recipient)
	{
		send_new_broadcast(&recipient, sender->username, message);
	});
}

/// Resends every broadcast the peer reports as missing, oldest first.
void Server::process_peer_ack(const Session* session, const uint8_t peerLast, const uint64_t peerBitmap)
{
	if (session == nullptr)
		return;

	const PeerAck ack{.last = peerLast, .received = peerBitmap};
	const auto lastSent = static_cast<uint8_t>(session->nextServerMessageId - 1);

	// The peer claims to have received something we never sent: ignore it.
	if (within(forward_distance(lastSent, ack.last), 1, 127))
	{
		logger.log_debug("Client received a message that was never sent.");
		return;
	}

	std::vector<StoredMessage> missing;

	for (const auto& message : session->history)
	{
		if (message.valid && is_missing_for_peer(message.id, ack))
			missing.push_back(message);
	}

	const auto age = [&](const StoredMessage& message)
	{
		return forward_distance(message.id, lastSent);
	};

	std::ranges::sort(missing,
	                  [&](const StoredMessage& a, const StoredMessage& b) { return age(a) > age(b); });

	for (const auto& message : missing)
		send_message_broadcast(session, message.id, message.username, message.text);
}

void Server::tick()
{
	const auto now = Clock::now();
	std::vector<uint8_t> timed_out;

	sessionManager.for_each_session([&](Session& session)
	{
		const auto idle = now - session.lastPacket;

		if (!session.suspicious && idle >= INACTIVITY_WARNING)
		{
			logger.log_debug("Session of user=" + std::to_string(session.userId) + " has become suspicious.");
			session.suspicious = true;
			session.nextHeartbeatRequest = now;
		}

		if (!session.suspicious)
			return;

		if (idle >= INACTIVITY_TIMEOUT)
		{
			logger.log_debug("Sending 'CONNECTION_CLOSED' to user=" + std::to_string(session.userId));
			send_user_packet(session.address, CONNECTION_CLOSED, SERVER_USER_IDENTIFIER);
			timed_out.push_back(session.userId);
			return;
		}

		if (now < session.nextHeartbeatRequest)
			return;

		logger.log_debug("Sending 'HEARTBEAT_REQUEST' to user=" + std::to_string(session.userId));
		send_receive_status(session.address, HEARTBEAT_REQUEST, session);
		session.nextHeartbeatRequest = now + HEARTBEAT_REQUEST_INTERVAL;
	});

	for (const auto id : timed_out)
	{
		logger.log_debug("Deleting session of user=" + std::to_string(id));
		sessionManager.delete_session(id);
	}
}
