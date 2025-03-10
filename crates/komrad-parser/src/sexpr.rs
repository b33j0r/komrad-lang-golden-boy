use komrad_ast::prelude::{BinaryOp, UnaryOp};
use owo_colors::OwoColorize;

/// Our S‑expression type.
#[derive(Debug, Clone)]
pub enum Sexpr {
    Skip,
    Atom(String),
    List(Vec<Sexpr>),
}

impl Sexpr {
    /// Pretty-print the S-expression tree using Tree-sitter style.
    pub fn format(&self, indent: usize) -> String {
        match self {
            Sexpr::Skip => String::new(),
            Sexpr::Atom(s) => s.clone(),
            Sexpr::List(items) => {
                let indent_str = "  ".repeat(indent);
                if items.is_empty() {
                    "()".to_string()
                } else {
                    let first = &items[0];

                    match first {
                        Sexpr::Atom(first_atom) => {
                            let mut result = format!("{}{}", "(".red(), first_atom.bright_cyan());
                            for item in &items[1..] {
                                match item {
                                    Sexpr::Skip => {}
                                    Sexpr::Atom(atom) => {
                                        result.push_str(&format!(" {}", atom));
                                    }
                                    Sexpr::List(_) => {
                                        result.push_str(&format!("\n{}", item.format(indent + 1)));
                                    }
                                }
                            }
                            result.push_str(&")".red().to_string());
                            format!("{}{}", indent_str, result)
                        }
                        _ => {
                            let inner = items
                                .iter()
                                .map(|item| item.format(indent + 1))
                                .collect::<Vec<_>>()
                                .join("\n");
                            format!("{}(\n{}\n{})", indent_str, inner, indent_str)
                        }
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for Sexpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(0))
    }
}

/// Conversion trait for S-expressions.
pub trait ToSexpr {
    fn to_sexpr(&self) -> Sexpr;
}

impl ToSexpr for UnaryOp {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            UnaryOp::Not => Sexpr::Atom("!".into()),
            UnaryOp::Neg => Sexpr::Atom("-".into()),
            UnaryOp::Inc => Sexpr::Atom("++".into()),
            UnaryOp::Dec => Sexpr::Atom("--".into()),
        }
    }
}

impl ToSexpr for BinaryOp {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            BinaryOp::Add => Sexpr::Atom("+".into()),
            BinaryOp::Sub => Sexpr::Atom("-".into()),
            BinaryOp::Mul => Sexpr::Atom("*".into()),
            BinaryOp::Div => Sexpr::Atom("/".into()),
            BinaryOp::And => Sexpr::Atom("&&".into()),
            BinaryOp::Or => Sexpr::Atom("||".into()),
        }
    }
}
