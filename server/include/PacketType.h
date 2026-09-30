#ifndef SERVER_PACKETTYPE_H
#define SERVER_PACKETTYPE_H

/**
 * List of every packet type supported
 */
enum PacketType : unsigned char
{
	UNDEFINED = 0x0,
	CONNECTION = 0x1,
	CONNECTION_ACCEPTED = 0x2,
	CONNECTION_TERMINATE = 0x3,
	// UNUSED = 0x4 - 0xF
	MESSAGE = 0x10,
	// NACK = 0x11
	// UNUSED = 0x12 - 0x1F
	HEARTBEAT = 0x20,
	HEARTBEAT_RECEIVED = 0x21,
	HEARTBEAT_REQUEST = 0x22,
};

#endif //SERVER_PACKETTYPE_H
