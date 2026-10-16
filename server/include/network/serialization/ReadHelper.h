#ifndef SERVER_READHELPER_H
#define SERVER_READHELPER_H

#include <cstddef>
#include <cstdint>

/// Reads the given bits from the buffer
uint8_t readVariableU8(const uint8_t* buffer, std::size_t& offset, std::size_t size);

/// Reads the given bits from the buffer
uint16_t readVariableU16(const uint8_t* buffer, std::size_t& offset, std::size_t size);

/// Reads the given bits from the buffer
uint32_t readVariableU32(const uint8_t* buffer, std::size_t& offset, std::size_t size);

/// Reads the given bits from the buffer
uint64_t readVariableU64(const uint8_t* buffer, std::size_t& offset, std::size_t size);

#endif //SERVER_READHELPER_H
