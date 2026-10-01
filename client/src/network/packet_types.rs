#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PacketType {
    Undefined = 0x0,
    Connection = 0x1,
    ConnectionAccepted = 0x2,
    ConnectionRejected = 0x3,
    ConnectionTerminate = 0x4,
    // UNUSED = 0x5 - 0xF
    Message = 0x10,
    // NACK = 0x11
    // UNUSED = 0x12 - 0x1F
    Heartbeat = 0x20,
    HeartbeatReceived = 0x21,
    HeartbeatRequest = 0x22,
}
