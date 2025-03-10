use komrad_ast::prelude::ParserError;

pub fn parse_module_to_builder(
    source: &str,
    source_file: Option<PathBuf>,
) -> Result<ModuleBuilder, ParserError> {
    let mut parser = Parser::new(source, source_file);
    parser.parse()?;
    Ok(parser.module_builder())
}
