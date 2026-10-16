// UDP chat server implementing the server-side behavior described by RFC.md.
// Linux/POSIX, C++17.
// Build: g++ -std=c++17 -Wall -Wextra -Wpedantic -O2 chat_server.cpp -o chat-server
// Run:   ./chat-server [port]

#include <unistd.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/select.h>
#include <sys/socket.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace protocol
{
	constexpr uint8_t CONNECTION_REQUEST = 0x01;
	constexpr uint8_t CONNECTION_ACCEPTED = 0x02;
	constexpr uint8_t CONNECTION_REJECTED = 0x03;
	constexpr uint8_t CONNECTION_CLOSED = 0x04;
	constexpr uint8_t MESSAGE_SUBMIT = 0x10;
	constexpr uint8_t MESSAGE_BROADCAST = 0x11;
	constexpr uint8_t MESSAGE_REQUEST = 0x12;
	constexpr uint8_t HEARTBEAT_BROADCAST = 0x20;
	constexpr uint8_t HEARTBEAT_RECEIVED = 0x21;
	constexpr uint8_t HEARTBEAT_REQUEST = 0x22;

	constexpr std::size_t MAX_DATAGRAM = 2048;
	constexpr std::size_t MAX_USERNAME = 16;
	constexpr std::size_t MAX_MESSAGE = 990;
	constexpr std::size_t HISTORY_SIZE = 64;
	using Clock = std::chrono::steady_clock;

	namespace
	{
		// Protocol fields are packed most-significant-bit first. Packets are sent as
		// whole bytes; any unused bits at the end of a packet are zero padding.
		class BitReader
		{
		public:
			BitReader(const uint8_t* data, const std::size_t size) : data_(data), bits_(size * 8)
			{
			}

			bool get(const std::size_t offset, const unsigned width, uint64_t& value) const
			{
				if (width > 64 || offset > bits_ || width > bits_ - offset) return false;
				value = 0;
				for (unsigned i = 0; i < width; ++i)
				{
					const auto bit = offset + i;
					value = (value << 1) | ((data_[bit / 8] >> (7 - bit % 8)) & 1u);
				}
				return true;
			}

			[[nodiscard]] std::size_t size_bits() const { return bits_; }

		private:
			const uint8_t* data_;
			std::size_t bits_;
		};

		class BitWriter
		{
		public:
			explicit BitWriter(const std::size_t bits) : data_((bits + 7) / 8, 0), bits_(bits)
			{
			}

			bool put(const std::size_t offset, const unsigned width, const uint64_t value)
			{
				if (width > 64 || offset > bits_ || width > bits_ - offset) return false;
				for (unsigned i = 0; i < width; ++i)
				{
					const auto bit = offset + i;
					const auto b = static_cast<uint8_t>((value >> (width - 1 - i)) & 1u);
					const auto mask = static_cast<uint8_t>(1u << (7 - bit % 8));
					if (b) data_[bit / 8] |= mask;
					else data_[bit / 8] &= static_cast<uint8_t>(~mask);
				}
				return true;
			}

			bool put_bytes(const std::size_t offset, const std::string& value)
			{
				for (std::size_t i = 0; i < value.size(); ++i)
					if (!put(offset + i * 8, 8, static_cast<uint8_t>(value[i]))) return false;
				return true;
			}

			[[nodiscard]] const std::vector<uint8_t>& bytes() const { return data_; }

		private:
			std::vector<uint8_t> data_;
			std::size_t bits_;
		};

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
				BitWriter w(16);
				w.put(0, 8, type);
				w.put(8, 6, userId);
				send_packet(to, w.bytes());
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
				auto it = sessions_.find(userId);
				if (it == sessions_.end() || !same_endpoint(it->second.address, peer)) return nullptr;
				return &it->second;
			}

			static bool valid_ascii(const std::string& s)
			{
				return std::ranges::all_of(s, [](const unsigned char c) { return c <= 0x7F; });
			}

			static bool read_ascii(const BitReader& r, const std::size_t offset, const std::size_t length,
			                       std::string& out)
			{
				out.clear();
				out.reserve(length);
				for (std::size_t i = 0; i < length; ++i)
				{
					uint64_t ch = 0;
					if (!r.get(offset + i * 8, 8, ch) || ch > 0x7F) return false;
					out.push_back(static_cast<char>(ch));
				}
				return true;
			}

			static std::size_t padded_bits(const std::size_t bits) { return (bits + 7) / 8 * 8; }

			void handle_packet(const uint8_t* data, const std::size_t size, const sockaddr_in& peer)
			{
				if (size == 0) return;
				const BitReader r(data, size);
				uint64_t raw = 0;
				if (!r.get(0, 8, raw) || raw == 0) return;
				switch (static_cast<uint8_t>(raw))
				{
				case CONNECTION_REQUEST: handle_connection_request(r, peer);
					break;
				case CONNECTION_CLOSED: handle_connection_closed(r, peer);
					break;
				case MESSAGE_SUBMIT: handle_message_submit(r, peer);
					break;
				case MESSAGE_REQUEST: handle_message_request(r, peer);
					break;
				case HEARTBEAT_BROADCAST: handle_heartbeat(r, peer);
					break;
				default: break; // unsupported types are ignored
				}
			}

			void handle_connection_request(const BitReader& r, const sockaddr_in& peer)
			{
				uint64_t clientId = 0, lengthMinusOne = 0;
				if (!r.get(8, 4, clientId) || !r.get(12, 4, lengthMinusOne)) return;
				const std::size_t n = lengthMinusOne + 1;
				if (n > MAX_USERNAME || r.size_bits() != padded_bits(16 + n * 8)) return;
				std::string name;
				if (!read_ascii(r, 16, n, name) || !valid_ascii(name)) return;

				if (const auto existing = find_session(peer, static_cast<uint8_t>(clientId)))
				{
					auto& s = sessions_.at(*existing);
					// Retries must return the original user ID, even if the username differs.
					s.lastPacket = Clock::now();
					s.suspicious = false;
					send_user_packet(peer, CONNECTION_ACCEPTED, s.userId);
					return;
				}
				const uint8_t id = allocate_user_id();
				if (!id)
				{
					send_type(peer, CONNECTION_REJECTED);
					return;
				}
				Session s;
				s.address = peer;
				s.clientId = static_cast<uint8_t>(clientId);
				s.userId = id;
				s.username = name;
				s.lastPacket = Clock::now();
				sessions_.emplace(id, std::move(s));
				send_user_packet(peer, CONNECTION_ACCEPTED, id);
				std::cout << name << " connected (user id " << static_cast<unsigned>(id) << ")\n";
			}

			void handle_connection_closed(const BitReader& r, const sockaddr_in& peer)
			{
				uint64_t id = 0;
				if (r.size_bits() != 16 || !r.get(8, 6, id) || id == 0) return;
				Session* s = authenticated_session(static_cast<uint8_t>(id), peer);
				if (!s) return;
				std::cout << s->username << " disconnected\n";
				sessions_.erase(static_cast<unsigned>(id));
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
			void process_peer_ack(Session& s, const uint8_t peerLast, const uint64_t peerBitmap) const
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

			void handle_heartbeat(const BitReader& r, const sockaddr_in& peer)
			{
				uint64_t id = 0, peerLast = 0, peerBitmap = 0;
				if (r.size_bits() != 88 || !r.get(8, 6, id) || !r.get(14, 8, peerLast) ||
					!r.get(22, 64, peerBitmap))
					return;
				Session* s = authenticated_session(static_cast<uint8_t>(id), peer);
				if (!s) return;
				mark_alive(*s);
				process_peer_ack(*s, static_cast<uint8_t>(peerLast), peerBitmap);
				send_receive_status(peer, HEARTBEAT_RECEIVED, *s);
			}

			void handle_message_request(const BitReader& r, const sockaddr_in& peer)
			{
				uint64_t id = 0, peerLast = 0, peerBitmap = 0;
				if (r.size_bits() != 88 || !r.get(8, 6, id) || !r.get(14, 8, peerLast) ||
					!r.get(22, 64, peerBitmap))
					return;
				Session* s = authenticated_session(static_cast<uint8_t>(id), peer);
				if (!s) return;
				mark_alive(*s);
				process_peer_ack(*s, static_cast<uint8_t>(peerLast), peerBitmap);
			}

			static void mark_alive(Session& s)
			{
				s.lastPacket = Clock::now();
				s.suspicious = false;
			}

			void send_receive_status(const sockaddr_in& to, const uint8_t type, const Session& s) const
			{
				BitWriter w(80);
				w.put(0, 8, type);
				w.put(8, 8, s.lastClientMessageId);
				w.put(16, 64, s.receivedClientMessages);
				send_packet(to, w.bytes());
			}

			void handle_message_submit(const BitReader& r, const sockaddr_in& peer)
			{
				uint64_t id = 0, peerLast = 0, peerBitmap = 0, messageId = 0, messageLength = 0;
				if (!r.get(8, 6, id) || !r.get(14, 8, peerLast) || !r.get(22, 64, peerBitmap) ||
					!r.get(86, 8, messageId) || !r.get(94, 10, messageLength))
					return;
				if (messageLength == 0 || messageLength > MAX_MESSAGE ||
					r.size_bits() != padded_bits(104 + messageLength * 8))
					return;
				Session* sender = authenticated_session(static_cast<uint8_t>(id), peer);
				if (!sender) return;
				std::string message;
				if (!read_ascii(r, 104, messageLength, message) || !valid_ascii(message))
					return;

				mark_alive(*sender);
				process_peer_ack(*sender, static_cast<uint8_t>(peerLast), peerBitmap);
				if (!identify_client_message(*sender, static_cast<uint8_t>(messageId))) return;

				const std::string username = sender->username;
				std::cout << username << ": " << message << "\n";
				// Each recipient has an independent server-side message counter/history.
				for (auto& [recipientId, recipient] : sessions_)
				{
					(void)recipientId;
					send_new_broadcast(recipient, username, message);
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
						send_user_packet(s.address, CONNECTION_CLOSED, s.userId);
						std::cout << s.username << " timed out\n";
						remove.push_back(id);
						continue;
					}
					if (s.suspicious && now >= s.nextHeartbeatRequest)
					{
						// HEARTBEAT_REQUEST reports the server's receive state for this client.
						BitWriter w(80);
						w.put(0, 8, HEARTBEAT_REQUEST);
						w.put(8, 8, s.lastClientMessageId);
						w.put(16, 64, s.receivedClientMessages);
						send_packet(s.address, w.bytes());
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
			// RFC's variable-length USERNAME_CONTENT and MESSAGE_CONTENT fields.
			const std::size_t bits = 8 + 8 + 8 + 64 + 4 + stored.username.size() * 8 +
				10 + stored.text.size() * 8;
			BitWriter w(bits);
			w.put(0, 8, MESSAGE_BROADCAST);
			w.put(8, 8, stored.id);
			w.put(16, 8, recipient.lastClientMessageId);
			w.put(24, 64, recipient.receivedClientMessages);
			w.put(88, 4, stored.username.size() - 1);
			w.put_bytes(92, stored.username);
			const std::size_t lengthOffset = 92 + stored.username.size() * 8;
			w.put(lengthOffset, 10, stored.text.size());
			w.put_bytes(lengthOffset + 10, stored.text);
			send_packet(recipient.address, w.bytes());
		}
	};
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
