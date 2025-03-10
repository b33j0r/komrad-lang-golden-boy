use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, Error, Diagnostic, PartialEq, Eq)]
#[error("{kind}")]
pub struct ParserError {
    /// The specific kind of error encountered.
    pub kind: ParseErrorKind,

    /// The span in the source code where the error occurred.
    #[label("{label}")]
    pub span: SourceSpan,

    /// Dynamic label message for this specific error.
    pub label: String,

    /// The source code associated with the error.
    #[source_code]
    pub src: String,
}

impl ParserError {
    pub fn from_kind(kind: ParseErrorKind, src: String, span: SourceSpan) -> Self {
        Self {
            kind,
            span,
            label: "Error occurred here.".to_string(), // Default label
            src,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ParseErrorKind {
    #[error("Invalid syntax")]
    InvalidSyntax,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Unexpected end of input")]
    UnexpectedEndOfInput,

    #[error("Invalid character: {0}")]
    InvalidCharacter(char),

    #[error("Invalid number format")]
    InvalidNumberFormat,

    #[error("Invalid string format")]
    InvalidStringFormat,

    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("Invalid operator: {0}")]
    InvalidOperator(String),

    #[error("Invalid type: {0}")]
    InvalidType(String),
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("Failed to send message")]
    SendError,
    #[error("Failed to receive message")]
    ReceiveError,
    #[error("Failed to parse message")]
    ParseError(ParserError),
    #[error("Division by zero")]
    DivisionByZero,
}
