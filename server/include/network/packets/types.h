#ifndef SERVER_TYPES_H
#define SERVER_TYPES_H

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

#endif //SERVER_TYPES_H
