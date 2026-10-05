use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    Failure = 1,
    InvalidArguments = 2,
    Connection = 3,
    Authentication = 4,
    Validation = 5,
    Conflict = 6,
    NotFound = 7,
    Integration = 8,
    Plugin = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Generic,
    InvalidArguments,
    Connection,
    Authentication,
    Validation,
    RevisionConflict,
    NotFound,
    Integration,
    Plugin,
}

impl ErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::Generic => "generic_error",
            ErrorKind::InvalidArguments => "invalid_arguments",
            ErrorKind::Connection => "connection_failed",
            ErrorKind::Authentication => "authentication_failed",
            ErrorKind::Validation => "validation_failed",
            ErrorKind::RevisionConflict => "revision_conflict",
            ErrorKind::NotFound => "not_found",
            ErrorKind::Integration => "integration_failed",
            ErrorKind::Plugin => "plugin_failed",
        }
    }
    pub fn exit(self) -> ExitCode {
        match self {
            ErrorKind::Generic => ExitCode::Failure,
            ErrorKind::InvalidArguments => ExitCode::InvalidArguments,
            ErrorKind::Connection => ExitCode::Connection,
            ErrorKind::Authentication => ExitCode::Authentication,
            ErrorKind::Validation => ExitCode::Validation,
            ErrorKind::RevisionConflict => ExitCode::Conflict,
            ErrorKind::NotFound => ExitCode::NotFound,
            ErrorKind::Integration => ExitCode::Integration,
            ErrorKind::Plugin => ExitCode::Plugin,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AdminError {
    kind: ErrorKind,
    message: String,
    details: Option<Value>,
}

impl AdminError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }
    pub fn with_details(kind: ErrorKind, message: impl Into<String>, details: Value) -> Self {
        Self {
            kind,
            message: message.into(),
            details: Some(details),
        }
    }
    pub fn invalid_arguments(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArguments, message)
    }
    pub fn connection(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Connection, message)
    }
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }
    pub fn set_details(&mut self, details: Value) {
        self.details = Some(details);
    }
    pub fn into_kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn attach_details(self, details: Value) -> Self {
        Self {
            details: Some(details),
            ..self
        }
    }
    pub fn info(&self) -> ErrorInfo {
        ErrorInfo {
            code: self.kind.code(),
            message: self.message.clone(),
            details: self.details.clone(),
        }
    }
    pub fn from_server_code(code: &str, message: impl Into<String>, status: u16) -> Self {
        let kind = match code {
            "conflict" => ErrorKind::RevisionConflict,
            "unauthorized" | "forbidden" => ErrorKind::Authentication,
            "invalid_input" | "unsupported_schema" | "unsupported_platform" => {
                ErrorKind::Validation
            }
            "not_found" => ErrorKind::NotFound,
            _ => kind_from_status(status),
        };
        Self::new(kind, message)
    }
    pub fn from_domain(error: crate::domain::Error) -> Self {
        let kind = match error.code {
            crate::contracts::ErrorCode::Conflict => ErrorKind::RevisionConflict,
            crate::contracts::ErrorCode::Unauthorized | crate::contracts::ErrorCode::Forbidden => {
                ErrorKind::Authentication
            }
            crate::contracts::ErrorCode::InvalidInput
            | crate::contracts::ErrorCode::UnsupportedSchema
            | crate::contracts::ErrorCode::UnsupportedPlatform => ErrorKind::Validation,
            _ => ErrorKind::Generic,
        };
        Self::new(kind, error.message.clone())
    }
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AdminError {}

pub fn kind_from_status(status: u16) -> ErrorKind {
    match status {
        401 | 403 => ErrorKind::Authentication,
        404 => ErrorKind::NotFound,
        409 => ErrorKind::RevisionConflict,
        400 | 405 | 415 | 422 => ErrorKind::Validation,
        _ => ErrorKind::Generic,
    }
}

pub type Result<T> = std::result::Result<T, AdminError>;
