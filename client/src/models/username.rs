use crate::network::packet::FieldError;
use std::fmt;

/// Longest username, in bytes
const MAX_USERNAME_LEN: usize = 16;

/// Structure used to define the name of a user
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username {
    content: String,
}

impl Username {
    pub fn new(name: impl Into<String>) -> Result<Self, FieldError> {
        let name = name.into();

        if name.is_empty() {
            return Err(FieldError::Empty);
        }

        if !name.is_ascii() {
            return Err(FieldError::NotAscii);
        }

        if name.len() > MAX_USERNAME_LEN {
            return Err(FieldError::TooLong {
                max: MAX_USERNAME_LEN,
            });
        }

        Ok(Self { content: name })
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.content)
    }
}
