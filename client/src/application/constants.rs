use std::time::Duration;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(200);
pub const INACTIVITY_WARNING: Duration = Duration::from_secs(1);
pub const SERVER_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(3);
pub const SERVER_CONNECTION_INTERVAL: Duration = Duration::from_secs(1);
pub const SERVER_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
