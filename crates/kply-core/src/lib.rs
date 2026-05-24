//! Core domain model for future Kply session primitives.

use std::fmt;

const SESSION_ID_MAX_LEN: usize = 63;

/// Stable identifier for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(String);

impl SessionId {
    /// Create a [`SessionId`] from a validated string value.
    pub fn new(value: impl Into<String>) -> Result<Self, SessionIdError> {
        let value = value.into();
        validate_session_id(&value)?;
        Ok(Self(value))
    }

    /// Borrow the session identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Error returned when a [`SessionId`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionIdError {
    /// Session identifiers cannot be empty.
    Empty,
    /// Session identifiers must fit common Kubernetes label value limits.
    TooLong { max_len: usize },
    /// Session identifiers must start and end with an ASCII letter or digit.
    InvalidBoundary,
    /// Session identifiers only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for SessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session id cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "session id cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => {
                formatter.write_str("session id must start and end with an ASCII letter or digit")
            }
            Self::InvalidCharacter { character } => write!(
                formatter,
                "session id contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for SessionIdError {}

fn validate_session_id(value: &str) -> Result<(), SessionIdError> {
    if value.is_empty() {
        return Err(SessionIdError::Empty);
    }

    if value.len() > SESSION_ID_MAX_LEN {
        return Err(SessionIdError::TooLong {
            max_len: SESSION_ID_MAX_LEN,
        });
    }

    let Some(first_character) = value.chars().next() else {
        return Err(SessionIdError::Empty);
    };
    let Some(last_character) = value.chars().next_back() else {
        return Err(SessionIdError::Empty);
    };

    if !first_character.is_ascii_alphanumeric() || !last_character.is_ascii_alphanumeric() {
        return Err(SessionIdError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_session_id_character(*character))
    {
        return Err(SessionIdError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_session_id_character(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
}

#[cfg(test)]
mod tests {
    use super::{SESSION_ID_MAX_LEN, SessionId, SessionIdError};

    #[test]
    fn creates_session_id_from_valid_value() {
        let session_id = SessionId::new("session-123").expect("session id should be valid");

        assert_eq!(session_id.as_str(), "session-123");
        assert_eq!(session_id.to_string(), "session-123");
    }

    #[test]
    fn rejects_empty_session_id() {
        let error = SessionId::new("").expect_err("empty session id should be rejected");

        assert_eq!(error, SessionIdError::Empty);
    }

    #[test]
    fn rejects_session_id_that_exceeds_max_length() {
        let value = "a".repeat(SESSION_ID_MAX_LEN + 1);
        let error = SessionId::new(value).expect_err("long session id should be rejected");

        assert_eq!(
            error,
            SessionIdError::TooLong {
                max_len: SESSION_ID_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_session_id_with_invalid_boundary() {
        let error = SessionId::new("-session").expect_err("leading hyphen should be rejected");

        assert_eq!(error, SessionIdError::InvalidBoundary);
    }

    #[test]
    fn rejects_session_id_with_invalid_character() {
        let error = SessionId::new("Session").expect_err("uppercase should be rejected");

        assert_eq!(error, SessionIdError::InvalidCharacter { character: 'S' });
    }
}
