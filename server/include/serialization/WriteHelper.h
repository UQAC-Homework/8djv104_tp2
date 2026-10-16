#ifndef SERVER_WRITEHELPER_H
#define SERVER_WRITEHELPER_H

#include <cstddef>
#include <cstdint>
#include <vector>

/// Writes the given bits into the buffer
void writeVariableU8(std::vector<uint8_t>& buffer, std::size_t& offset, uint8_t value, std::size_t size);

/// Writes the given bits into the buffer
void writeVariableU16(std::vector<uint8_t>& buffer, std::size_t& offset, uint16_t value, std::size_t size);

/// Writes the given bits into the buffer
void writeVariableU32(std::vector<uint8_t>& buffer, std::size_t& offset, uint32_t value, std::size_t size);

/// Writes the given bits into the buffer
void writeVariableU64(std::vector<uint8_t>& buffer, std::size_t& offset, uint64_t value, std::size_t size);

#endif //SERVER_WRITEHELPER_H
