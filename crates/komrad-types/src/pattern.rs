use crate::scope::Scope;
use crate::types::Expr;
use crate::Msg;
use std::fmt::Display;

#[derive(Debug, Clone, Hash)]
pub struct Pattern {
    terms: Vec<Expr>,
}

impl Pattern {
    pub fn new(terms: Vec<Expr>) -> Self {
        Self { terms }
    }

    pub fn terms(&self) -> Vec<Expr> {
        self.terms.clone()
    }

    /// Attempt to bind a message to this pattern, producing a Scope on success
    pub fn bind(&self, message: &Msg) -> Option<Scope> {
        let msg_parts = message.message.clone();

        if self.terms.len() != msg_parts.len() {
            return None; // Length mismatch, no match
        }

        let mut scope = Scope::new();

        for (pattern_term, msg_part) in self.terms.iter().zip(msg_parts) {
            match pattern_term {
                Expr::Variable(name) => {
                    // Variable captures the message part into scope
                    scope.set(name.clone(), msg_part.clone());
                }
                Expr::Value(val) => {
                    if &msg_part != val {
                        return None; // mismatch: not the same constant
                    }
                }
                // Add more cases for complex matching if needed
                _ => {
                    // Unsupported pattern term (could be expanded)
                    return None;
                }
            }
        }

        Some(scope)
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.terms == other.terms
    }
}

impl Eq for Pattern {}
