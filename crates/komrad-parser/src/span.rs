use komrad_ast::prelude::ParserError;
use miette::NamedSource;
use nom::IResult;
use nom_locate::LocatedSpan;
use std::sync::Arc;

pub type KResult<'a, O> = IResult<Span<'a>, O, ParserError>;

pub type Span<'input> = LocatedSpan<&'input str, Arc<NamedSource<String>>>;

pub fn new_span(input: &str) -> Span {
    Span::new_extra(
        input,
        Arc::new(NamedSource::new("source", input.to_string())),
    )
}

pub fn empty_span() -> Span<'static> {
    Span::new_extra("", Arc::new(NamedSource::new("source", "".to_string())))
}
