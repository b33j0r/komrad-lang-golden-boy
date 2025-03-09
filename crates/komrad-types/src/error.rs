use crate::Value;
use miette::{Diagnostic, SourceSpan};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error, Diagnostic)]
#[error("{kind}")]
pub struct ParserError {
    /// The specific kind of error encountered.
    pub kind: RuntimeError,

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
    pub fn from_kind(kind: RuntimeError, src: String, span: SourceSpan) -> Self {
        Self {
            kind,
            span,
            label: "Error occurred here.".to_string(), // Default label
            src,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum RuntimeError {
    #[error("Parse error: {0}")]
    ParseError(ParseErrorKind),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Value error: {0}")]
    ValueError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Not implemented")]
    NotImplemented(String),

    #[error("Timeout")]
    Timeout,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Another error occurred: {0}")]
    AnotherError(String),

    #[error("Division by zero: {0}/0")]
    DivisionByZero(Box<Value>),
}
