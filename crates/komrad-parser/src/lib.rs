// use miette::SourceSpan;
// use komrad_types::ParserError;
// use komrad_types::{ParseErrorKind, RuntimeError};

//
// fn main() {
//     // Example usage of the ParserError
//     let error_kind = RuntimeError::ParseError(ParseErrorKind::InvalidCharacter('a'));
//     let src = "let x = 42;\nlet y = x + ;\n".to_string();
//     let span = SourceSpan::new(12.into(), 1); // Example span
//     let error = ParserError::from_kind(error_kind, src.clone(), span);
//
//     let report = miette::Report::new(error);
//     println!("{:?}", report);
// }

pub mod error;
mod module_builder;
pub mod parse;
pub mod parser;
pub mod sexpr;
pub mod span;
