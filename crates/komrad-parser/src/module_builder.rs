use komrad_ast::prelude::Statement;
use std::path::PathBuf;
pub struct ModuleBuilder {
    name: String,
    source_file: Option<PathBuf>,
    statements: Vec<Statement>,
}

impl ModuleBuilder {
    pub fn new() -> Self {
        ModuleBuilder {
            name: String::new(),
            source_file: None,
            statements: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_source_file(&mut self, path: PathBuf) {
        self.source_file = Some(path);
    }

    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }
}
