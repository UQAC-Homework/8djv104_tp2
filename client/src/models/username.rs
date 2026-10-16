use crate::network::packet;
use crate::network::packet::{FieldError};
use std::fmt;

/// Longest username, in bytes
const MAX_USERNAME_LEN: usize = 16;

/// A username: 1 to 16 ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username {
    content: String,
}

impl Username {
    pub fn new(name: impl Into<String>) -> Result<Self, FieldError> {
        let name = name.into();
        packet::check_text(&name, MAX_USERNAME_LEN)?;
        Ok(Self { content: name })
    }
    
    pub fn bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.content)
    }
}
