#include <unistd.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/select.h>
#include <sys/socket.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

#include "include/network/packets/types.h"
#include "include/network/serialization/PackerWriter.h"
#include "include/network/serialization/PacketReader.h"

namespace protocol
{
	constexpr std::size_t MAX_DATAGRAM = 2048;
	constexpr std::size_t MAX_USERNAME = 16;
	constexpr std::size_t MAX_MESSAGE = 990;
	constexpr std::size_t HISTORY_SIZE = 64;
	using Clock = std::chrono::steady_clock;

	namespace
	{
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

			Clock::time_point lastPacket = Clock::now();
			bool suspicious = false;
			Clock::time_point nextHeartbeatRequest = Clock::now();
		};

		class Server
		{
		public:
			explicit Server(const uint16_t port) : port_(port)
			{
				socket_ = socket(AF_INET, SOCK_DGRAM, 0);
				if (socket_ < 0) throw std::runtime_error("socket() failed");
				sockaddr_in local{};
				local.sin_family = AF_INET;
				local.sin_addr.s_addr = htonl(INADDR_ANY);
				local.sin_port = htons(port_);
				if (bind(socket_, reinterpret_cast<sockaddr*>(&local), sizeof(local)) < 0)
				{
					close(socket_);
					socket_ = -1;
					throw std::runtime_error("bind() failed");
				}
			}

			~Server() { if (socket_ >= 0) close(socket_); }

			void run()
			{
				std::cout << "Listening on UDP port " << port_ << "\n";
				for (;;)
				{
					tick();
					fd_set set;
					FD_ZERO(&set);
					FD_SET(socket_, &set);
					timeval timeout{.tv_sec = 0, .tv_usec = 50000};
					const int ready = select(socket_ + 1, &set, nullptr, nullptr, &timeout);
					if (ready < 0)
					{
						if (errno == EINTR) continue;
						std::perror("select");
						continue;
					}
					if (ready == 0) continue;
					std::array<uint8_t, MAX_DATAGRAM> buffer{};
					sockaddr_in peer{};
					socklen_t peerLength = sizeof(peer);
					const ssize_t count = recvfrom(socket_, buffer.data(), buffer.size(), 0,
					                               reinterpret_cast<sockaddr*>(&peer), &peerLength);
					if (count > 0) handle_packet(buffer.data(), static_cast<std::size_t>(count), peer);
				}
			}

		private:
			int socket_ = -1;
			uint16_t port_;
			std::unordered_map<unsigned, Session> sessions_;

			void send_packet(const sockaddr_in& to, const std::vector<uint8_t>& bytes) const
			{
				(void)sendto(socket_, bytes.data(), bytes.size(), 0,
				             reinterpret_cast<const sockaddr*>(&to), sizeof(to));
			}

			void send_type(const sockaddr_in& to, const uint8_t type) const
			{
				send_packet(to, std::vector{type});
			}

			void send_user_packet(const sockaddr_in& to, const uint8_t type, const uint8_t userId) const
			{
				PacketWriter writer;
				writer.writeByte(type);
				writer.writeSextet(userId);
				send_packet(to, writer.getBuffer());
			}

			static bool same_endpoint(const sockaddr_in& a, const sockaddr_in& b)
			{
				return a.sin_family == b.sin_family && a.sin_port == b.sin_port &&
					a.sin_addr.s_addr == b.sin_addr.s_addr;
			}

			std::optional<unsigned> find_session(const sockaddr_in& peer, const uint8_t clientId) const
			{
				for (const auto& [id, s] : sessions_)
					if (same_endpoint(s.address, peer) && s.clientId == clientId) return id;
				return std::nullopt;
			}

			uint8_t allocate_user_id() const
			{
				for (uint8_t id = 1; id <= 63; ++id) if (!sessions_.contains(id)) return id;
				return 0;
			}

			Session* authenticated_session(const uint8_t userId, const sockaddr_in& peer)
			{
				const auto it = sessions_.find(userId);
				if (it == sessions_.end() || !same_endpoint(it->second.address, peer)) return nullptr;
				return &it->second;
			}

			void handle_packet(const uint8_t* data, const std::size_t size, const sockaddr_in& peer)
			{
				if (size == 0)
					return;

				PacketReader reader(data, size);

				try
				{
					const auto raw = reader.readByte();

					switch (raw)
					{
					case CONNECTION_REQUEST: handle_connection_request(reader, peer);
						break;
					case CONNECTION_CLOSED: handle_connection_closed(reader, peer);
						break;
					case MESSAGE_SUBMIT: handle_message_submit(reader, peer);
						break;
					case MESSAGE_REQUEST: handle_message_request(reader, peer);
						break;
					case HEARTBEAT_BROADCAST: handle_heartbeat(reader, peer);
						break;
					default: break; // unsupported types are ignored
					}
				}
				catch (std::exception& e)
				{
					std::cerr << "Error while handling packet: " << e.what() << std::endl;
				}
			}

			void handle_connection_request(PacketReader& reader, const sockaddr_in& peer)
			{
				const auto client_identifier = reader.readNybble();
				const auto username_length = reader.readNybble() + 1;

				if (username_length > MAX_USERNAME)
					return;

				const std::string name = reader.readString(username_length);

				const auto existing = find_session(peer, client_identifier);

				// Retries must return the original user ID, even if the username differs.
				if (existing.has_value())
				{
					auto& s = sessions_.at(*existing);
					s.lastPacket = Clock::now();
					s.suspicious = false;
					send_user_packet(peer, CONNECTION_ACCEPTED, s.userId);
					return;
				}

				const uint8_t id = allocate_user_id();

				if (id == 0)
				{
					send_type(peer, CONNECTION_REJECTED);
					return;
				}

				Session s;
				s.address = peer;
				s.clientId = client_identifier;
				s.userId = id;
				s.username = name;
				s.lastPacket = Clock::now();
				sessions_.emplace(id, std::move(s));
				send_user_packet(peer, CONNECTION_ACCEPTED, id);

				std::cout << "User '" << name << "' has connected (ID: " << id << ")" << std::endl;
			}

			void handle_connection_closed(PacketReader& reader, const sockaddr_in& peer)
			{
				const auto user_identifier = reader.readSextet();

				// Invalid user ID
				if (user_identifier == 0)
					return;

				const Session* s = authenticated_session(user_identifier, peer);

				if (s == nullptr)
					return;

				std::cout << "User '" << s->username << "' has disconnected." << std::endl;

				sessions_.erase(user_identifier);
			}

			// Update our view of messages submitted by the client. Bit zero describes
			// the identifier immediately before lastClientMessageId.
			static bool identify_client_message(Session& s, const uint8_t id)
			{
				const auto forward = static_cast<uint8_t>(id - s.lastClientMessageId);
				if (forward >= 1 && forward <= 127)
				{
					const auto previous = s.lastClientMessageId;
					if (forward >= 64) s.receivedClientMessages = 0;
					else s.receivedClientMessages <<= forward;
					const auto previousBit = static_cast<unsigned>(forward - 1);
					if (previousBit < 64) s.receivedClientMessages |= uint64_t{1} << previousBit;
					s.lastClientMessageId = id;
					(void)previous;
					return true;
				}
				const auto backward = static_cast<uint8_t>(s.lastClientMessageId - id);
				if (backward == 0 || backward > 64) return false;
				const uint64_t mask = uint64_t{1} << (backward - 1);
				if (s.receivedClientMessages & mask) return false;
				s.receivedClientMessages |= mask;
				return true; // late, but not previously received
			}

			// The peer's last/bitmap describe which server-to-peer messages arrived.
			// Retransmit only messages in the 64-slot history, oldest first.
			void process_peer_ack(const Session& s, const uint8_t peerLast, const uint64_t peerBitmap) const
			{
				const auto lastSent = static_cast<uint8_t>(s.nextServerMessageId - 1);
				const auto peerAhead = static_cast<uint8_t>(peerLast - lastSent);
				if (peerAhead >= 1 && peerAhead <= 127) return; // invalid/future acknowledgement

				std::vector<StoredMessage> missing;
				for (const auto& m : s.history)
				{
					if (!m.valid) continue;
					const auto ahead = static_cast<uint8_t>(m.id - peerLast);
					bool isMissing = false;
					if (ahead >= 1 && ahead <= 127)
					{
						// It was sent after the peer's reported last received message.
						isMissing = true;
					}
					else
					{
						const auto behind = static_cast<uint8_t>(peerLast - m.id);
						if (behind >= 1 && behind <= 64)
							isMissing = ((peerBitmap >> (behind - 1)) & 1u) == 0;
					}
					if (isMissing) missing.push_back(m);
				}
				const uint8_t newest = lastSent;
				std::ranges::sort(missing, [newest](const StoredMessage& a, const StoredMessage& b)
				{
					return static_cast<uint8_t>(newest - a.id) > static_cast<uint8_t>(newest - b.id);
				});
				for (const auto& m : missing) send_message_broadcast(s, m, true);
			}

			void handle_heartbeat(PacketReader& reader, const sockaddr_in& peer)
			{
				const auto user_identifier = reader.readSextet();
				const auto last_message_identifier = reader.readByte();
				const auto received_messages = reader.readQuadword();

				Session* s = authenticated_session(user_identifier, peer);

				if (s == nullptr)
					return;

				mark_alive(*s);
				process_peer_ack(*s, last_message_identifier, received_messages);
				send_receive_status(peer, HEARTBEAT_RECEIVED, *s);
			}

			void handle_message_request(PacketReader& reader, const sockaddr_in& peer)
			{
				const auto user_identifier = reader.readSextet();
				const auto last_message_identifier = reader.readByte();
				const auto received_messages = reader.readQuadword();

				Session* s = authenticated_session(user_identifier, peer);

				if (s == nullptr)
					return;

				mark_alive(*s);
				process_peer_ack(*s, last_message_identifier, received_messages);
			}

			static void mark_alive(Session& s)
			{
				s.lastPacket = Clock::now();
				s.suspicious = false;
			}

			void send_receive_status(const sockaddr_in& to, const uint8_t type, const Session& s) const
			{
				PacketWriter writer;
				writer.writeByte(type);
				writer.writeByte(s.lastClientMessageId);
				writer.writeQuadword(s.receivedClientMessages);
				send_packet(to, writer.getBuffer());
			}

			void handle_message_submit(PacketReader& reader, const sockaddr_in& peer)
			{
				const auto user_identifier = reader.readSextet();
				const auto last_message_identifier = reader.readByte();
				const auto received_messages = reader.readQuadword();
				const auto message_identifier = reader.readByte();
				const auto message_length = reader.readDeclet();

				if (message_length > MAX_MESSAGE)
					return;

				const auto message = reader.readString(message_length);

				Session* sender = authenticated_session(user_identifier, peer);

				if (sender == nullptr)
					return;

				mark_alive(*sender);
				process_peer_ack(*sender, last_message_identifier, received_messages);

				if (!identify_client_message(*sender, message_identifier))
					return;

				std::cout << "User '" << sender->username << "' has sent: '" << message << "'." << std::endl;

				// Each recipient has an independent server-side message counter/history.
				for (auto& [recipientId, recipient] : sessions_)
				{
					(void)recipientId;
					send_new_broadcast(recipient, sender->username, message);
				}
			}

			void send_new_broadcast(Session& recipient, const std::string& username, const std::string& message) const
			{
				StoredMessage stored;
				stored.valid = true;
				stored.id = recipient.nextServerMessageId;
				stored.username = username;
				stored.text = message;
				recipient.history[stored.id % HISTORY_SIZE] = stored; // cache before first send
				recipient.nextServerMessageId = static_cast<uint8_t>(recipient.nextServerMessageId + 1);
				send_message_broadcast(recipient, stored, false);
			}

			void send_message_broadcast(const Session& recipient, const StoredMessage& stored,
			                            bool retransmission) const;

			void tick()
			{
				const auto now = Clock::now();
				std::vector<unsigned> remove;
				for (auto& [id, s] : sessions_)
				{
					const auto idle = now - s.lastPacket;
					if (!s.suspicious && idle >= std::chrono::milliseconds(1500))
					{
						s.suspicious = true;
						s.nextHeartbeatRequest = now;
					}
					if (s.suspicious && idle >= std::chrono::seconds(3))
					{
						send_user_packet(s.address, CONNECTION_CLOSED, 0b000000);
						std::cout << s.username << " timed out\n";
						remove.push_back(id);
						continue;
					}
					if (s.suspicious && now >= s.nextHeartbeatRequest)
					{
						// HEARTBEAT_REQUEST reports the server's receive state for this client.
						PacketWriter writer;
						writer.writeByte(HEARTBEAT_REQUEST);
						writer.writeByte(s.lastClientMessageId);
						writer.writeQuadword(s.receivedClientMessages);
						send_packet(s.address, writer.getBuffer());
						s.nextHeartbeatRequest = now + std::chrono::milliseconds(200);
					}
				}
				for (unsigned id : remove) sessions_.erase(id);
			}
		};

		void Server::send_message_broadcast(const Session& recipient, const StoredMessage& stored,
		                                    const bool retransmission) const
		{
			(void)retransmission;

			if (stored.username.empty() || stored.username.size() > MAX_USERNAME ||
				stored.text.empty() || stored.text.size() > MAX_MESSAGE)
				return;

			PacketWriter writer;

			writer.writeByte(MESSAGE_BROADCAST);
			writer.writeByte(stored.id);
			writer.writeByte(recipient.lastClientMessageId);
			writer.writeQuadword(recipient.receivedClientMessages);
			writer.writeNybble(stored.username.size() - 1);
			writer.writeString(stored.username);
			writer.writeDeclet(stored.text.size());
			writer.writeString(stored.text);

			send_packet(recipient.address, writer.getBuffer());
		}
	}
} // namespace protocol

int main(const int argc, char** argv)
{
	uint16_t port = 4000;
	if (argc > 2)
	{
		std::cerr << "Usage: " << argv[0] << " [port]\n";
		return 1;
	}
	if (argc == 2)
	{
		char* end = nullptr;
		const long value = std::strtol(argv[1], &end, 10);
		if (!end || *end != '\0' || value < 1 || value > 65535)
		{
			std::cerr << "Invalid port\n";
			return 1;
		}
		port = static_cast<uint16_t>(value);
	}
	try
	{
		protocol::Server server(port);
		server.run();
	}
	catch (const std::exception& e)
	{
		std::cerr << "Server error: " << e.what() << "\n";
		return 1;
	}
}
