#ifndef SERVER_PACKERWRITER_H
#define SERVER_PACKERWRITER_H

/// Structure used to write packets to a binary format
struct PackerWriter
{
private:
	/// Buffer to write to
	const unsigned char buffer[1024];

	/// Length of the buffer in bits
	const unsigned long length;

	/// Offset from the start of the buffer in bits
	unsigned long offset;

	/// Checks if the writer can write the given amount of bits
	[[nodiscard]]
	bool canWrite(unsigned long size) const;

public:
	explicit PackerWriter(unsigned char buffer[], unsigned long length);

	/// Gets the buffer of the writer
	void buffer();

	/// Writes the given nybble
	void writeNybble(unsigned char value);

	/// Writes the given byte
	void writeByte(unsigned char value);
};


#endif //SERVER_PACKERWRITER_H
