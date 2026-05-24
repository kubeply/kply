//! Core domain model for future Kply session primitives.

use std::fmt;

const SESSION_TOKEN_MAX_LEN: usize = 63;
const WORKLOAD_KIND_MAX_LEN: usize = 63;
const ROUTE_HEADER_NAME_MAX_LEN: usize = 63;
const ROUTE_HEADER_VALUE_MAX_LEN: usize = 255;
const ROUTE_HOST_MAX_LEN: usize = 253;
const ROUTE_HOST_LABEL_MAX_LEN: usize = 63;

/// Maximum allowed length for an [`ImageRef`] value.
pub const IMAGE_REF_MAX_LEN: usize = 255;

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

/// Kubernetes workload target for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkloadRef {
    namespace: String,
    kind: String,
    name: String,
}

impl WorkloadRef {
    /// Create a [`WorkloadRef`] from validated namespace, kind, and name parts.
    pub fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, WorkloadRefError> {
        let namespace = namespace.into();
        let kind = kind.into();
        let name = name.into();

        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(WorkloadRefError::Namespace)?;
        validate_workload_kind(&kind).map_err(WorkloadRefError::Kind)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(WorkloadRefError::Name)?;

        Ok(Self {
            namespace,
            kind,
            name,
        })
    }

    /// Borrow the workload namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the workload kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Borrow the workload name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for WorkloadRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}/{}", self.namespace, self.kind, self.name)
    }
}

/// Container image reference proposed for a future sandbox workload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageRef(String);

impl ImageRef {
    /// Create an [`ImageRef`] from a validated image reference string.
    pub fn new(value: impl Into<String>) -> Result<Self, ImageRefError> {
        let value = value.into();
        validate_image_ref(&value)?;
        Ok(Self(value))
    }

    /// Borrow the image reference as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Traffic selector for routing future test requests to a sandbox workload.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RouteSelector {
    /// Match requests by HTTP header name and value.
    Header { name: String, value: String },
    /// Match requests by host name.
    Host { hostname: String },
}

impl RouteSelector {
    /// Create a header-based [`RouteSelector`].
    pub fn header(
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, RouteSelectorError> {
        let name = name.into();
        let value = value.into();

        validate_route_header_name(&name).map_err(RouteSelectorError::HeaderName)?;
        validate_route_header_value(&value).map_err(RouteSelectorError::HeaderValue)?;

        Ok(Self::Header { name, value })
    }

    /// Create a host-based [`RouteSelector`].
    pub fn host(hostname: impl Into<String>) -> Result<Self, RouteSelectorError> {
        let hostname = hostname.into();
        validate_route_host(&hostname).map_err(RouteSelectorError::Host)?;

        Ok(Self::Host { hostname })
    }

    /// Return the stable selector kind used in agent-readable output.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Header { .. } => "header",
            Self::Host { .. } => "host",
        }
    }

    /// Borrow the header selector parts when this selector matches by header.
    pub fn header_parts(&self) -> Option<(&str, &str)> {
        match self {
            Self::Header { name, value } => Some((name, value)),
            Self::Host { .. } => None,
        }
    }

    /// Borrow the host name when this selector matches by host.
    pub fn hostname(&self) -> Option<&str> {
        match self {
            Self::Header { .. } => None,
            Self::Host { hostname } => Some(hostname),
        }
    }
}

impl fmt::Display for RouteSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header { name, value } => write!(formatter, "header:{name}={value}"),
            Self::Host { hostname } => write!(formatter, "host:{hostname}"),
        }
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

/// Error returned when an [`ImageRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRefError {
    /// Image references cannot be empty.
    Empty,
    /// Image references must stay bounded for stable reports and labels.
    TooLong { max_len: usize },
    /// Image references must include a non-empty image name component.
    MissingName,
    /// Image references must start and end with an ASCII letter, digit, or digest value.
    InvalidBoundary,
    /// Image references only allow ASCII image reference characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for ImageRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("image ref cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "image ref cannot exceed {max_len} characters")
            }
            Self::MissingName => formatter.write_str("image ref must include an image name"),
            Self::InvalidBoundary => {
                formatter.write_str("image ref has an invalid boundary character")
            }
            Self::InvalidCharacter { character } => {
                write!(
                    formatter,
                    "image ref contains invalid character `{character}`"
                )
            }
        }
    }
}

impl std::error::Error for ImageRefError {}

/// Error returned when a [`RouteSelector`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSelectorError {
    /// Header selector names must be valid HTTP field names.
    HeaderName(RouteHeaderNameError),
    /// Header selector values must be printable ASCII values.
    HeaderValue(RouteHeaderValueError),
    /// Host selectors must be lowercase DNS host names.
    Host(RouteHostError),
}

impl fmt::Display for RouteSelectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderName(error) => write!(formatter, "invalid route header name: {error}"),
            Self::HeaderValue(error) => write!(formatter, "invalid route header value: {error}"),
            Self::Host(error) => write!(formatter, "invalid route host: {error}"),
        }
    }
}

impl std::error::Error for RouteSelectorError {}

/// Error returned when a route header name is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHeaderNameError {
    /// Header names cannot be empty.
    Empty,
    /// Header names must stay bounded for stable reports.
    TooLong { max_len: usize },
    /// Header names only allow ASCII HTTP token characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHeaderNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route header name cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "route header name cannot exceed {max_len} characters"
                )
            }
            Self::InvalidCharacter { character } => write!(
                formatter,
                "route header name contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for RouteHeaderNameError {}

/// Error returned when a route header value is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHeaderValueError {
    /// Header values cannot be empty.
    Empty,
    /// Header values must stay bounded for stable reports.
    TooLong { max_len: usize },
    /// Header values only allow visible ASCII characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHeaderValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route header value cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "route header value cannot exceed {max_len} characters"
                )
            }
            Self::InvalidCharacter { character } => write!(
                formatter,
                "route header value contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for RouteHeaderValueError {}

/// Error returned when a route host is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHostError {
    /// Hosts cannot be empty.
    Empty,
    /// Hosts must stay within DNS host length limits.
    TooLong { max_len: usize },
    /// Host labels must stay within DNS label length limits.
    LabelTooLong { max_len: usize },
    /// Hosts cannot contain empty labels.
    EmptyLabel,
    /// Hosts must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Hosts only allow lowercase ASCII letters, digits, dots, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route host cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "route host cannot exceed {max_len} characters")
            }
            Self::LabelTooLong { max_len } => {
                write!(
                    formatter,
                    "route host label cannot exceed {max_len} characters"
                )
            }
            Self::EmptyLabel => formatter.write_str("route host cannot contain empty labels"),
            Self::InvalidBoundary => formatter
                .write_str("route host must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => {
                write!(
                    formatter,
                    "route host contains invalid character `{character}`"
                )
            }
        }
    }
}

impl std::error::Error for RouteHostError {}

/// Error returned when a [`WorkloadRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadRefError {
    /// Workload namespaces use the same token rules as session names and identifiers.
    Namespace(WorkloadTokenError),
    /// Workload kinds must be non-empty Kubernetes-style kind identifiers.
    Kind(WorkloadKindError),
    /// Workload names use the same token rules as session names and identifiers.
    Name(WorkloadTokenError),
}

impl fmt::Display for WorkloadRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid workload namespace: {error}")
            }
            Self::Kind(error) => write!(formatter, "invalid workload kind: {error}"),
            Self::Name(error) => write!(formatter, "invalid workload name: {error}"),
        }
    }
}

impl std::error::Error for WorkloadRefError {}

/// Error returned when a workload namespace or name is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadTokenError {
    /// Workload namespace and name values cannot be empty.
    Empty,
    /// Workload namespace and name values must fit common Kubernetes name limits.
    TooLong { max_len: usize },
    /// Workload namespace and name values must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Workload namespace and name values only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for WorkloadTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "value cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter
                .write_str("value must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => {
                write!(formatter, "value contains invalid character `{character}`")
            }
        }
    }
}

impl std::error::Error for WorkloadTokenError {}

/// Error returned when a workload kind is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadKindError {
    /// Workload kinds cannot be empty.
    Empty,
    /// Workload kinds must stay bounded for stable reports and labels.
    TooLong { max_len: usize },
    /// Workload kinds must start and end with an ASCII letter or digit.
    InvalidBoundary,
    /// Workload kinds only allow ASCII letters, digits, dots, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for WorkloadKindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("workload kind cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "workload kind cannot exceed {max_len} characters"
                )
            }
            Self::InvalidBoundary => formatter
                .write_str("workload kind must start and end with an ASCII letter or digit"),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "workload kind contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for WorkloadKindError {}

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

impl From<SessionTokenError> for WorkloadTokenError {
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

fn validate_route_header_name(value: &str) -> Result<(), RouteHeaderNameError> {
    if value.is_empty() {
        return Err(RouteHeaderNameError::Empty);
    }

    if value.len() > ROUTE_HEADER_NAME_MAX_LEN {
        return Err(RouteHeaderNameError::TooLong {
            max_len: ROUTE_HEADER_NAME_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_header_name_character(*character))
    {
        return Err(RouteHeaderNameError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_route_header_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

fn validate_route_header_value(value: &str) -> Result<(), RouteHeaderValueError> {
    if value.is_empty() {
        return Err(RouteHeaderValueError::Empty);
    }

    if value.len() > ROUTE_HEADER_VALUE_MAX_LEN {
        return Err(RouteHeaderValueError::TooLong {
            max_len: ROUTE_HEADER_VALUE_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_header_value_character(*character))
    {
        return Err(RouteHeaderValueError::InvalidCharacter { character });
    }

    Ok(())
}

// Route selectors use deterministic token-like header values, so spaces are
// rejected even though HTTP permits broader field values.
fn is_route_header_value_character(character: char) -> bool {
    character.is_ascii_graphic()
}

fn validate_route_host(value: &str) -> Result<(), RouteHostError> {
    if value.is_empty() {
        return Err(RouteHostError::Empty);
    }

    if value.len() > ROUTE_HOST_MAX_LEN {
        return Err(RouteHostError::TooLong {
            max_len: ROUTE_HOST_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(RouteHostError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_route_host_boundary(first_character) || !is_route_host_boundary(last_character) {
        return Err(RouteHostError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_host_character(*character))
    {
        return Err(RouteHostError::InvalidCharacter { character });
    }

    for label in value.split('.') {
        if label.is_empty() {
            return Err(RouteHostError::EmptyLabel);
        }

        if label.len() > ROUTE_HOST_LABEL_MAX_LEN {
            return Err(RouteHostError::LabelTooLong {
                max_len: ROUTE_HOST_LABEL_MAX_LEN,
            });
        }

        let mut label_characters = label.chars();
        let label_first_character = label_characters.next().ok_or(RouteHostError::EmptyLabel)?;
        let label_last_character = label_characters
            .next_back()
            .unwrap_or(label_first_character);

        if !is_route_host_boundary(label_first_character)
            || !is_route_host_boundary(label_last_character)
        {
            return Err(RouteHostError::InvalidBoundary);
        }
    }

    Ok(())
}

fn is_route_host_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || character == '-'
        || character == '.'
}

fn is_route_host_boundary(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

fn validate_image_ref(value: &str) -> Result<(), ImageRefError> {
    if value.is_empty() {
        return Err(ImageRefError::Empty);
    }

    if value.len() > IMAGE_REF_MAX_LEN {
        return Err(ImageRefError::TooLong {
            max_len: IMAGE_REF_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(ImageRefError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_image_ref_boundary(first_character) || !is_image_ref_boundary(last_character) {
        return Err(ImageRefError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_image_ref_character(*character))
    {
        return Err(ImageRefError::InvalidCharacter { character });
    }

    if value
        .split(['/', ':', '@'])
        .any(|component| component.is_empty())
    {
        return Err(ImageRefError::MissingName);
    }

    validate_image_repository_components(value)?;

    Ok(())
}

fn is_image_ref_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/' | ':' | '@')
}

fn is_image_ref_repository_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '.' | '_' | '-')
}

fn is_image_registry_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '.' | '-' | ':')
}

fn is_image_ref_boundary(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

fn validate_image_repository_components(value: &str) -> Result<(), ImageRefError> {
    let image_without_digest = value.split('@').next().unwrap_or(value);
    let components = image_without_digest.split('/').collect::<Vec<_>>();
    let last_component_index = components.len().saturating_sub(1);

    for (index, component) in components.iter().enumerate() {
        let component = if index == last_component_index {
            component.split(':').next().unwrap_or(component)
        } else {
            component
        };

        let valid_character = if index == 0 && is_registry_component(component) {
            is_image_registry_character
        } else {
            is_image_ref_repository_character
        };

        if let Some(character) = component
            .chars()
            .find(|character| !valid_character(*character))
        {
            return Err(ImageRefError::InvalidCharacter { character });
        }
    }

    Ok(())
}

fn is_registry_component(component: &str) -> bool {
    component == "localhost" || component.contains('.') || component.contains(':')
}

fn validate_workload_kind(value: &str) -> Result<(), WorkloadKindError> {
    if value.is_empty() {
        return Err(WorkloadKindError::Empty);
    }

    if value.len() > WORKLOAD_KIND_MAX_LEN {
        return Err(WorkloadKindError::TooLong {
            max_len: WORKLOAD_KIND_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(WorkloadKindError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !first_character.is_ascii_alphanumeric() || !last_character.is_ascii_alphanumeric() {
        return Err(WorkloadKindError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_workload_kind_character(*character))
    {
        return Err(WorkloadKindError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_workload_kind_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '.' || character == '-'
}

#[cfg(test)]
mod tests {
    use super::{
        IMAGE_REF_MAX_LEN, ImageRef, ImageRefError, ROUTE_HEADER_NAME_MAX_LEN,
        ROUTE_HEADER_VALUE_MAX_LEN, ROUTE_HOST_LABEL_MAX_LEN, ROUTE_HOST_MAX_LEN,
        RouteHeaderNameError, RouteHeaderValueError, RouteHostError, RouteSelector,
        RouteSelectorError, SESSION_TOKEN_MAX_LEN, SessionId, SessionIdError, SessionName,
        SessionNameError, SessionStatus, WORKLOAD_KIND_MAX_LEN, WorkloadKindError, WorkloadRef,
        WorkloadRefError, WorkloadTokenError,
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

    #[test]
    fn creates_header_route_selector() {
        let selector =
            RouteSelector::header("x-kply-session", "session-123").expect("route selector");

        assert_eq!(selector.kind(), "header");
        assert_eq!(
            selector.header_parts(),
            Some(("x-kply-session", "session-123"))
        );
        assert_eq!(selector.hostname(), None);
        assert_eq!(selector.to_string(), "header:x-kply-session=session-123");
    }

    #[test]
    fn creates_header_route_selector_with_exact_max_value_length() {
        let value = "a".repeat(ROUTE_HEADER_VALUE_MAX_LEN);
        let selector =
            RouteSelector::header("x-kply-session", value.as_str()).expect("route selector");

        assert_eq!(
            selector.header_parts(),
            Some(("x-kply-session", value.as_str()))
        );
    }

    #[test]
    fn creates_header_route_selector_with_special_token_characters() {
        for name in ["x_kply", "x.kply", "x+kply", "x~kply", "x!#$%&'*^`|kply"] {
            let selector = RouteSelector::header(name, "session-123").expect("route selector");

            assert_eq!(selector.header_parts(), Some((name, "session-123")));
        }
    }

    #[test]
    fn creates_host_route_selector() {
        let selector =
            RouteSelector::host("session-123.preview.example.com").expect("route selector");

        assert_eq!(selector.kind(), "host");
        assert_eq!(selector.header_parts(), None);
        assert_eq!(selector.hostname(), Some("session-123.preview.example.com"));
        assert_eq!(selector.to_string(), "host:session-123.preview.example.com");
    }

    #[test]
    fn creates_host_route_selector_with_exact_max_length() {
        let label = "a".repeat(ROUTE_HOST_LABEL_MAX_LEN);
        let final_label = "a".repeat(ROUTE_HOST_LABEL_MAX_LEN - 2);
        let host = format!("{label}.{label}.{label}.{final_label}");
        assert_eq!(host.len(), ROUTE_HOST_MAX_LEN);

        let selector = RouteSelector::host(host.as_str()).expect("route selector");

        assert_eq!(selector.hostname(), Some(host.as_str()));
    }

    #[test]
    fn rejects_empty_route_header_name() {
        let error = RouteSelector::header("", "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::Empty)
        );
    }

    #[test]
    fn rejects_long_route_header_name() {
        let name = "a".repeat(ROUTE_HEADER_NAME_MAX_LEN + 1);
        let error = RouteSelector::header(name, "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::TooLong {
                max_len: ROUTE_HEADER_NAME_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_invalid_route_header_name_character() {
        let error =
            RouteSelector::header("x kply session", "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::InvalidCharacter {
                character: ' '
            })
        );
    }

    #[test]
    fn rejects_empty_route_header_value() {
        let error = RouteSelector::header("x-kply-session", "").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::Empty)
        );
    }

    #[test]
    fn rejects_long_route_header_value() {
        let value = "a".repeat(ROUTE_HEADER_VALUE_MAX_LEN + 1);
        let error = RouteSelector::header("x-kply-session", value).expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::TooLong {
                max_len: ROUTE_HEADER_VALUE_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_control_route_header_value_character() {
        let error =
            RouteSelector::header("x-kply-session", "session\n123").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::InvalidCharacter {
                character: '\n'
            })
        );
    }

    #[test]
    fn rejects_space_route_header_value_character() {
        let error =
            RouteSelector::header("x-kply-session", "session 123").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::InvalidCharacter {
                character: ' '
            })
        );
    }

    #[test]
    fn rejects_empty_route_host() {
        let error = RouteSelector::host("").expect_err("host");

        assert_eq!(error, RouteSelectorError::Host(RouteHostError::Empty));
    }

    #[test]
    fn rejects_long_route_host() {
        let host = format!("{}.example.com", "a".repeat(ROUTE_HOST_MAX_LEN));
        let error = RouteSelector::host(host).expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::TooLong {
                max_len: ROUTE_HOST_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_long_route_host_label() {
        let host = format!("{}.example.com", "a".repeat(ROUTE_HOST_LABEL_MAX_LEN + 1));
        let error = RouteSelector::host(host).expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::LabelTooLong {
                max_len: ROUTE_HOST_LABEL_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_route_host_empty_label() {
        let error = RouteSelector::host("session..example.com").expect_err("host");

        assert_eq!(error, RouteSelectorError::Host(RouteHostError::EmptyLabel));
    }

    #[test]
    fn rejects_route_host_invalid_boundary() {
        for host in ["-session.example.com", "session-.example.com"] {
            let error = RouteSelector::host(host).expect_err("host");

            assert_eq!(
                error,
                RouteSelectorError::Host(RouteHostError::InvalidBoundary)
            );
        }
    }

    #[test]
    fn rejects_route_host_invalid_character() {
        let error = RouteSelector::host("session.exa_mple.com").expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_workload_ref_from_valid_parts() {
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");

        assert_eq!(workload.namespace(), "checkout");
        assert_eq!(workload.kind(), "Deployment");
        assert_eq!(workload.name(), "checkout-api");
        assert_eq!(workload.to_string(), "checkout/Deployment/checkout-api");
    }

    #[test]
    fn creates_workload_ref_for_custom_resource_kind() {
        let workload = WorkloadRef::new("platform", "Rollout.argoproj.io", "api-rollout")
            .expect("workload ref");

        assert_eq!(workload.kind(), "Rollout.argoproj.io");
    }

    #[test]
    fn rejects_workload_ref_with_invalid_namespace() {
        let error =
            WorkloadRef::new("Checkout", "Deployment", "checkout-api").expect_err("namespace");

        assert_eq!(
            error,
            WorkloadRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_name() {
        let error = WorkloadRef::new("checkout", "Deployment", "checkout_api").expect_err("name");

        assert_eq!(
            error,
            WorkloadRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_kind_boundary() {
        let error =
            WorkloadRef::new("checkout", "-Deployment", "checkout-api").expect_err("kind boundary");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::InvalidBoundary)
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_kind_character() {
        let error = WorkloadRef::new("checkout", "Deploy_ment", "checkout-api").expect_err("kind");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_workload_ref_with_long_kind() {
        let kind = "A".repeat(WORKLOAD_KIND_MAX_LEN + 1);
        let error = WorkloadRef::new("checkout", kind, "checkout-api").expect_err("long kind");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::TooLong {
                max_len: WORKLOAD_KIND_MAX_LEN
            })
        );
    }

    #[test]
    fn creates_image_ref_from_tagged_reference() {
        let image_ref =
            ImageRef::new("registry.example.com/platform/checkout-api:1.2.3").expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api:1.2.3"
        );
        assert_eq!(
            image_ref.to_string(),
            "registry.example.com/platform/checkout-api:1.2.3"
        );
    }

    #[test]
    fn creates_image_ref_from_simple_name() {
        let image_ref = ImageRef::new("nginx").expect("image ref");

        assert_eq!(image_ref.as_str(), "nginx");
    }

    #[test]
    fn creates_image_ref_from_library_path() {
        let image_ref = ImageRef::new("library/nginx:latest").expect("image ref");

        assert_eq!(image_ref.as_str(), "library/nginx:latest");
    }

    #[test]
    fn creates_image_ref_with_repository_underscore() {
        let image_ref = ImageRef::new("my_image:v1").expect("image ref");

        assert_eq!(image_ref.as_str(), "my_image:v1");
    }

    #[test]
    fn creates_image_ref_from_deep_repository_path() {
        let image_ref = ImageRef::new("registry.io/a/b/c/image:tag").expect("image ref");

        assert_eq!(image_ref.as_str(), "registry.io/a/b/c/image:tag");
    }

    #[test]
    fn creates_image_ref_from_digest_reference() {
        let image_ref = ImageRef::new("registry.example.com/platform/checkout-api@sha256:abcdef")
            .expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api@sha256:abcdef"
        );
    }

    #[test]
    fn creates_image_ref_with_tag_and_digest() {
        let image = "registry.example.com/platform/checkout-api:1.2.3@sha256:abcdef";
        let image_ref = ImageRef::new(image).expect("image ref");

        assert_eq!(image_ref.as_str(), image);
    }

    #[test]
    fn creates_image_ref_with_registry_port() {
        let image_ref = ImageRef::new("localhost:5000/platform/checkout-api:dev")
            .expect("image ref with registry port");

        assert_eq!(
            image_ref.as_str(),
            "localhost:5000/platform/checkout-api:dev"
        );
    }

    #[test]
    fn creates_image_ref_with_mixed_case_tag() {
        let image_ref =
            ImageRef::new("registry.example.com/platform/checkout-api:BuildA").expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api:BuildA"
        );
    }

    #[test]
    fn rejects_empty_image_ref() {
        let error = ImageRef::new("").expect_err("empty image ref should be rejected");

        assert_eq!(error, ImageRefError::Empty);
    }

    #[test]
    fn rejects_long_image_ref() {
        let value = "a".repeat(IMAGE_REF_MAX_LEN + 1);
        let error = ImageRef::new(value).expect_err("long image ref should be rejected");

        assert_eq!(
            error,
            ImageRefError::TooLong {
                max_len: IMAGE_REF_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_image_ref_with_invalid_boundary() {
        for value in ["/checkout-api:1.2.3", "checkout-api:"] {
            let error = ImageRef::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, ImageRefError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_image_ref_with_invalid_character() {
        let error = ImageRef::new("checkout api:1.2.3").expect_err("space should be rejected");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: ' ' });
    }

    #[test]
    fn rejects_image_ref_with_uppercase_repository() {
        let error =
            ImageRef::new("registry.example.com/platform/Checkout-api:1.2.3").expect_err("image");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: 'C' });
    }

    #[test]
    fn rejects_uppercase_in_path_after_registry_port() {
        let error = ImageRef::new("localhost:5000/Platform/checkout-api:1.2.3").expect_err("image");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: 'P' });
    }

    #[test]
    fn rejects_image_ref_with_empty_component() {
        let error = ImageRef::new("registry.example.com//checkout-api:1.2.3")
            .expect_err("empty path component should be rejected");

        assert_eq!(error, ImageRefError::MissingName);
    }
}
