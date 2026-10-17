#ifndef SERVER_SERVER_H
#define SERVER_SERVER_H
#include <vector>
#include <netinet/in.h>

#include "SessionManager.h"
#include "../Logger.h"
#include "../LossSimulator.h"
#include "serialization/PacketReader.h"


enum class ArrivalKind { New, Late, Ignored };

struct Arrival
{
	ArrivalKind kind;
	uint8_t distance = 0;
};

class Server
{
public:
	explicit Server(uint16_t port, Logger& logger, LossSimulator& lossSimulator);

	~Server();

	void run();

private:
	int socket = -1;
	uint16_t port;
	SessionManager sessionManager;
	Logger& logger;
	LossSimulator& lossSimulator;

	void send_packet(const sockaddr_in& to, const std::vector<uint8_t>& bytes) const;

	void send_type(const sockaddr_in& to, uint8_t type) const;

	void send_user_packet(const sockaddr_in& to, uint8_t type, uint8_t userId) const;

	void handle_packet(const uint8_t* data, std::size_t size, const sockaddr_in& peer);

	void handle_connection_request(PacketReader& reader, const sockaddr_in& peer);

	void handle_connection_closed(PacketReader& reader);

	// The peer's last/bitmap describe which server-to-peer messages arrived.
	// Retransmit only messages in the 64-slot history, oldest first.
	void process_peer_ack(const Session* session, uint8_t peerLast, uint64_t peerBitmap) const;

	void handle_heartbeat(PacketReader& reader, const sockaddr_in& peer);

	void handle_message_request(PacketReader& reader);

	void send_receive_status(const sockaddr_in& to, uint8_t type, const Session& s) const;

	void handle_message_submit(PacketReader& reader);

	void send_new_broadcast(Session* recipient, const std::string& username, const std::string& message) const;

	void send_message_broadcast(const Session* recipient, uint8_t message_identifier, const std::string& username,
	                            const std::string& message) const;

	void send_message_request(const Session* recipient) const;

	void tick();
};


#endif //SERVER_SERVER_H
