use std::sync::Arc;
use miette::NamedSource;
use nom_locate::LocatedSpan;

pub type Span<'input> = LocatedSpan<&'input str, Arc<NamedSource<String>>>;

pub fn new_span(input: &str) -> Span {
    Span::new_extra(
        input,
        Arc::new(NamedSource::new("source", input.to_string())),
    )
}

pub fn empty_span() -> Span<'static> {
    Span::new_extra(
        "",
        Arc::new(NamedSource::new("source", "".to_string())),
    )
}