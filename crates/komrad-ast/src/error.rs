use miette::{Diagnostic, NamedSource, SourceSpan};
use nom::error::{ErrorKind as NomErrorKind, FromExternalError, ParseError as NomParseError};
use nom_locate::LocatedSpan;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub type Span<'a> = LocatedSpan<&'a str, Arc<NamedSource<String>>>;

/// Our master parser error type.
///
/// Implements Nom’s ParseError so errors carry a full source for Miette reporting.
#[derive(Clone, Debug, Error, Diagnostic, Eq, PartialEq, Hash)]
#[error("{kind}")]
pub struct ParserError {
    #[source_code]
    pub src: Arc<NamedSource<String>>,

    #[label("Error occurred here.")]
    pub span: SourceSpan,

    pub kind: ErrorKind,
}

impl Serialize for ParserError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ParserError", 3)?;
        state.serialize_field("src", &self.src.name())?;
        state.serialize_field("kind", &self.kind)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ParserError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ParserErrorData {
            src: String,
            kind: ErrorKind,
        }

        let data = ParserErrorData::deserialize(deserializer)?;
        let src = Arc::new(NamedSource::new(data.src.clone(), "".to_string()));
        Ok(ParserError {
            src,
            span: SourceSpan::new(0.into(), data.src.len().into()),
            kind: data.kind,
        })
    }
}

impl ParserError {
    /// Construct a new ParseError by extracting the full source from the span’s extra.
    pub fn new(kind: ErrorKind, input: Span<'_>) -> Self {
        let offset = input.location_offset();
        let len = input.fragment().len().max(1); // ensure at least length 1

        Self {
            src: input.extra.clone(),
            span: SourceSpan::new(offset.into(), len.into()),
            kind,
        }
    }
}

impl<'a> NomParseError<Span<'a>> for ParserError {
    fn from_error_kind(input: Span<'a>, _kind: NomErrorKind) -> Self {
        Self::new(
            ErrorKind::UnexpectedToken(input.fragment().to_string()),
            input,
        )
    }

    fn append(_input: Span<'a>, _kind: NomErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: Span<'a>, _c: char) -> Self {
        Self::new(
            ErrorKind::UnexpectedToken(input.fragment().to_string()),
            input,
        )
    }
}

impl<'a, E> FromExternalError<Span<'a>, E> for ParserError
where
    E: std::fmt::Display + std::fmt::Debug,
{
    fn from_external_error(input: Span<'a>, _kind: NomErrorKind, _e: E) -> Self {
        Self::new(ErrorKind::InvalidSyntax, input)
    }
}

/// Helper function to build an "empty" Span for unexpected EOF or similar errors.
pub fn empty_span() -> Span<'static> {
    Span::new_extra("", Arc::new(NamedSource::new("<empty>", "".to_string())))
}

#[derive(Debug, Clone, Error, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    #[error("Invalid syntax")]
    InvalidSyntax,

    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),

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

    #[error("Unexpected end of block")]
    UnexpectedEndOfEmbeddedBlock,
}

#[derive(Debug, Clone, Error, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeError {
    #[error("Failed to parse message")]
    ParseError(ParserError),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Invalid arguments: {0}")]
    InvalidArugments(String),

    #[error("Name not found in scope: {0}")]
    NameNotFound(String),

    #[error("Failed to send message")]
    SendError,

    #[error("Failed to receive message")]
    ReceiveError,

    #[error("Failed to send control message")]
    SendControlError,

    #[error("Failed to receive control message")]
    ReceiveControlError,

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid agent definition")]
    InvalidAgentDefinition,

    #[error("Agent not registered: {0}")]
    AgentNotRegistered(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    #[error("No handler found for message: {0}")]
    HandlerNotFound(String),

    #[error("External service error")]
    ExternalServiceError,
}
