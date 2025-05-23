use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use nom::{Parser, character::complete::line_ending};

/// Parse a blank line as a NoOp statement.
pub(crate) fn parse_blank_line(input: Span) -> KResult<Statement> {
    let (remaining, _) = line_ending.parse(input)?;
    Ok((remaining, Statement::NoOp))
}

/// Parse a comment line, e.g. `// hello`.
pub(crate) fn parse_comment(input: Span) -> KResult<Statement> {
    use nom::{
        Parser, bytes::complete::tag, character::complete::not_line_ending, sequence::preceded,
    };

    let (remaining, comment_content) = preceded(tag("//"), not_line_ending).parse(input)?;
    let comment_str = comment_content.fragment().to_string();

    Ok((remaining, Statement::Comment(comment_str)))
}
