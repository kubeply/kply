//! Core domain model for future Kply session primitives.

use std::fmt;

const SESSION_TOKEN_MAX_LEN: usize = 63;

/// Stable identifier for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(String);

impl SessionId {
    /// Create a [`SessionId`] from a validated string value.
    pub fn new(value: impl Into<String>) -> Result<Self, SessionIdError> {
        let value = value.into();
        validate_session_token(&value)?;
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

/// Stable user-facing name for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionName(String);

impl SessionName {
    /// Create a [`SessionName`] from a validated string value.
    pub fn new(value: impl Into<String>) -> Result<Self, SessionNameError> {
        let value = value.into();
        validate_session_token(&value)?;
        Ok(Self(value))
    }

    /// Borrow the session name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Lifecycle status for a future Kply session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionStatus {
    /// Session inputs have been accepted but no cluster preparation has started.
    Planned,
    /// Kply is preparing sandbox resources or route isolation.
    Preparing,
    /// The sandbox session is available for agent or test traffic.
    Active,
    /// Kply is running checks against the active session.
    Verifying,
    /// The session cannot proceed until an explicit issue is resolved.
    Blocked,
    /// The session passed checks and is ready for promotion or human approval.
    Ready,
    /// Kply has removed the temporary session resources.
    CleanedUp,
    /// The session failed and requires inspection.
    Failed,
}

impl SessionStatus {
    /// Return every session lifecycle status in declaration order, including terminal states.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Planned,
            Self::Preparing,
            Self::Active,
            Self::Verifying,
            Self::Blocked,
            Self::Ready,
            Self::CleanedUp,
            Self::Failed,
        ]
    }

    /// Return the stable snake_case status name used in agent-readable output.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Preparing => "preparing",
            Self::Active => "active",
            Self::Verifying => "verifying",
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::CleanedUp => "cleaned_up",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for SessionStatus {
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
    /// Session identifiers must start and end with a lowercase ASCII letter or digit.
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
            Self::InvalidBoundary => formatter
                .write_str("session id must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "session id contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for SessionIdError {}

/// Error returned when a [`SessionName`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionNameError {
    /// Session names cannot be empty.
    Empty,
    /// Session names must fit common Kubernetes label value limits.
    TooLong { max_len: usize },
    /// Session names must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Session names only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for SessionNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session name cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "session name cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter.write_str(
                "session name must start and end with a lowercase ASCII letter or digit",
            ),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "session name contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for SessionNameError {}

impl From<SessionTokenError> for SessionIdError {
    fn from(error: SessionTokenError) -> Self {
        match error {
            SessionTokenError::Empty => Self::Empty,
            SessionTokenError::TooLong { max_len } => Self::TooLong { max_len },
            SessionTokenError::InvalidBoundary => Self::InvalidBoundary,
            SessionTokenError::InvalidCharacter { character } => {
                Self::InvalidCharacter { character }
            }
        }
    }
}

impl From<SessionTokenError> for SessionNameError {
    fn from(error: SessionTokenError) -> Self {
        match error {
            SessionTokenError::Empty => Self::Empty,
            SessionTokenError::TooLong { max_len } => Self::TooLong { max_len },
            SessionTokenError::InvalidBoundary => Self::InvalidBoundary,
            SessionTokenError::InvalidCharacter { character } => {
                Self::InvalidCharacter { character }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionTokenError {
    Empty,
    TooLong { max_len: usize },
    InvalidBoundary,
    InvalidCharacter { character: char },
}

fn validate_session_token(value: &str) -> Result<(), SessionTokenError> {
    if value.is_empty() {
        return Err(SessionTokenError::Empty);
    }

    if value.len() > SESSION_TOKEN_MAX_LEN {
        return Err(SessionTokenError::TooLong {
            max_len: SESSION_TOKEN_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(SessionTokenError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_session_token_boundary(first_character) || !is_session_token_boundary(last_character) {
        return Err(SessionTokenError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_session_token_character(*character))
    {
        return Err(SessionTokenError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_session_token_character(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
}

fn is_session_token_boundary(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{
        SESSION_TOKEN_MAX_LEN, SessionId, SessionIdError, SessionName, SessionNameError,
        SessionStatus,
    };

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
        let value = "a".repeat(SESSION_TOKEN_MAX_LEN + 1);
        let error = SessionId::new(value).expect_err("long session id should be rejected");

        assert_eq!(
            error,
            SessionIdError::TooLong {
                max_len: SESSION_TOKEN_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_session_id_with_invalid_boundary() {
        for value in ["-session", "Session", "session-"] {
            let error = SessionId::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, SessionIdError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_session_id_with_invalid_character() {
        let error = SessionId::new("sesSion").expect_err("uppercase should be rejected");

        assert_eq!(error, SessionIdError::InvalidCharacter { character: 'S' });
    }

    #[test]
    fn creates_session_name_from_valid_value() {
        let session_name = SessionName::new("checkout-test").expect("session name should be valid");

        assert_eq!(session_name.as_str(), "checkout-test");
        assert_eq!(session_name.to_string(), "checkout-test");
    }

    #[test]
    fn rejects_empty_session_name() {
        let error = SessionName::new("").expect_err("empty session name should be rejected");

        assert_eq!(error, SessionNameError::Empty);
    }

    #[test]
    fn rejects_session_name_that_exceeds_max_length() {
        let value = "a".repeat(SESSION_TOKEN_MAX_LEN + 1);
        let error = SessionName::new(value).expect_err("long session name should be rejected");

        assert_eq!(
            error,
            SessionNameError::TooLong {
                max_len: SESSION_TOKEN_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_session_name_with_invalid_boundary() {
        for value in ["-checkout", "Checkout", "checkout-"] {
            let error = SessionName::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, SessionNameError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_session_name_with_invalid_character() {
        let error = SessionName::new("check_out").expect_err("underscore should be rejected");

        assert_eq!(error, SessionNameError::InvalidCharacter { character: '_' });
    }

    #[test]
    fn lists_session_statuses_in_lifecycle_order() {
        assert_eq!(
            SessionStatus::all(),
            &[
                SessionStatus::Planned,
                SessionStatus::Preparing,
                SessionStatus::Active,
                SessionStatus::Verifying,
                SessionStatus::Blocked,
                SessionStatus::Ready,
                SessionStatus::CleanedUp,
                SessionStatus::Failed,
            ]
        );
    }

    #[test]
    fn renders_session_status_names() {
        let status_names = SessionStatus::all()
            .iter()
            .map(SessionStatus::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            status_names,
            [
                "planned",
                "preparing",
                "active",
                "verifying",
                "blocked",
                "ready",
                "cleaned_up",
                "failed",
            ]
        );
        assert_eq!(SessionStatus::CleanedUp.to_string(), "cleaned_up");
    }
}
