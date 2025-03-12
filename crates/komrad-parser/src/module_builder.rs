use komrad_ast::prelude::{Block, Statement};
use komrad_ast::sexpr::{Sexpr, ToSexpr};
use std::path::PathBuf;

#[derive(Debug, Clone)]
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

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn build_block(&self) -> Block {
        Block::new(self.statements.clone())
    }
}

// pub enum Sexpr {
//     Skip,
//     Atom(String),
//     List(Vec<Sexpr>),
// }

impl ToSexpr for ModuleBuilder {
    fn to_sexpr(&self) -> Sexpr {
        let mut sexpr = Vec::new();
        if !self.name.is_empty() {
            if let Some(source_file) = &self.source_file {
                sexpr.push(Sexpr::Atom("module".to_string()));
                sexpr.push(Sexpr::Atom(self.name.clone()));
                sexpr.push(Sexpr::Atom(source_file.to_string_lossy().to_string()));
            } else {
                sexpr.push(Sexpr::Atom("module".to_string()));
                sexpr.push(Sexpr::Atom(self.name.clone()));
            }
        }
        for statement in &self.statements {
            sexpr.push(statement.to_sexpr());
        }
        Sexpr::List(sexpr)
    }
}
