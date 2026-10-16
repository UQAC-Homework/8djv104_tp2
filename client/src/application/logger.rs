use std::fmt::{Arguments, Display};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

/// List of possible levels of log
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        };
        write!(f, "{}", str)
    }
}

/// Minimum level of log to show
static MIN_LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

/// Sets the minimum level of log
pub fn set_level(level: Level) {
    MIN_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Checks if the given level is enabled
fn is_enabled(level: Level) -> bool {
    level as u8 >= MIN_LEVEL.load(Ordering::Relaxed)
}

fn get_time() -> Result<String, SystemTimeError> {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH)?;

    let total_secs = duration.as_secs();
    let milliseconds = duration.subsec_millis();

    let seconds = total_secs % 60;
    let minutes = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;

    let time_string = format!(
        "{:02}:{:02}:{:02}:{:03}",
        hours, minutes, seconds, milliseconds
    );

    Ok(time_string)
}

pub fn log(level: Level, args: Arguments) {
    if !is_enabled(level) {
        return;
    }

    let time = get_time().unwrap_or("???".parse().unwrap());

    let line = format!("[{time}] [{:<5}] {args}", level.to_string());

    if level == Level::Error {
        eprintln!("{}", line);
    } else {
        println!("{}", line);
    }
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Debug, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Info, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Warn, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::log($crate::logger::Level::Error, format_args!($($arg)*))
    };
}
