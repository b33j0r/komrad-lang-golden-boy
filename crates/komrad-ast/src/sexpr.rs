use crate::prelude::{BinaryOp, Block, Expr, Handler, Pattern, Statement, UnaryOp, ValueType};
use crate::type_expr::TypeExpr;
use crate::value::Value;
use owo_colors::OwoColorize;
use std::fmt::Write;

/// Represents an S-expression, which is used for debugging and
/// displaying the AST in a readable format.
#[derive(Debug, Clone)]
pub enum Sexpr {
    Skip,
    Atom(String),
    List(Vec<Sexpr>),
}

impl Sexpr {
    pub fn format(&self, indent: usize) -> String {
        match self {
            Sexpr::Skip => String::new(),
            Sexpr::Atom(text) => text.clone(),
            Sexpr::List(items) if items.is_empty() => "()".to_owned(),
            Sexpr::List(items) => {
                // Helper for indentation
                let indent_str = " ".repeat(indent);

                // For multi-line lists, we start with "(" on the same line
                // and place each item, one per line, with proper indentation.
                let mut result = "(".red().to_string();
                for (i, item) in items.iter().enumerate() {
                    if i == 0 {
                        // First item, just add a space
                        write!(result, "{}", item.format(indent + 2).bright_cyan()).ok();
                    } else {
                        // Subsequent items go on their own line
                        write!(result, "\n{}  {}", indent_str, item.format(indent + 2)).ok();
                    }
                }
                // Close the list
                let closing_parent = ")".red();
                write!(result, "{}", closing_parent).ok();
                result
            }
        }
    }
}

impl std::fmt::Display for Sexpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(0))
    }
}

/// Trait for converting a value to an S-expression
pub trait ToSexpr {
    fn to_sexpr(&self) -> Sexpr;
}

impl ToSexpr for UnaryOp {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            UnaryOp::Neg => Sexpr::Atom("negate".to_string()),
            UnaryOp::Not => Sexpr::Atom("not".to_string()),
            UnaryOp::Inc => Sexpr::Atom("add1".to_string()),
            UnaryOp::Dec => Sexpr::Atom("sub1".to_string()),
        }
    }
}

impl ToSexpr for BinaryOp {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            BinaryOp::Add => Sexpr::Atom("+".to_string()),
            BinaryOp::Sub => Sexpr::Atom("-".to_string()),
            BinaryOp::Mul => Sexpr::Atom("*".to_string()),
            BinaryOp::Div => Sexpr::Atom("/".to_string()),
            BinaryOp::Mod => Sexpr::Atom("%".to_string()),
            BinaryOp::And => Sexpr::Atom("and".to_string()),
            BinaryOp::Or => Sexpr::Atom("or".to_string()),
        }
    }
}

impl ToSexpr for Value {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Value::Empty => Sexpr::List(vec![Sexpr::Atom("empty".to_string())]),
            Value::Error(err) => Sexpr::List(vec![
                Sexpr::Atom("error".to_string()),
                Sexpr::Atom(err.to_string()),
            ]),
            Value::Channel(_) => Sexpr::List(vec![Sexpr::Atom("channel".to_string())]),
            Value::Boolean(b) => Sexpr::List(vec![
                Sexpr::Atom("bool".to_string()),
                Sexpr::Atom(b.to_string()),
            ]),
            Value::Word(w) => Sexpr::Atom(w.clone()),
            Value::String(s) => Sexpr::Atom(format!("\"{}\"", s)),
            Value::Number(n) => Sexpr::List(vec![
                Sexpr::Atom("number".to_string()),
                Sexpr::Atom(n.to_string()),
            ]),
            Value::List(items) => {
                let mut list_items = vec![Sexpr::Atom("list".to_string())];
                list_items.extend(items.iter().map(|item| item.to_sexpr()));
                Sexpr::List(list_items)
            }
            Value::Block(block) => {
                Sexpr::List(vec![Sexpr::Atom("block".to_string()), block.to_sexpr()])
            }
            Value::Bytes(bytes) => Sexpr::List(vec![
                Sexpr::Atom("bytes".to_string()),
                Sexpr::Atom(format!("{:?}", bytes)),
            ]),
            Value::EmbeddedBlock(eb) => {
                let mut tags = vec![Sexpr::Atom("tags".to_string())];
                tags.extend(eb.tags().iter().map(|t| Sexpr::Atom(t.clone())));

                Sexpr::List(vec![
                    Sexpr::Atom("embedded-block".to_string()),
                    Sexpr::List(tags),
                    Sexpr::List(vec![
                        Sexpr::Atom("text".to_string()),
                        Sexpr::Atom(format!("\"{}\"", eb.text())),
                    ]),
                ])
            }
        }
    }
}

impl ToSexpr for Expr {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Expr::Value(value) => value.to_sexpr(),
            Expr::Variable(name) => Sexpr::Atom(name.clone()),
            Expr::Binary(binary) => {
                let left = binary.left().to_sexpr();
                let op = binary.operator().to_sexpr();
                let right = binary.right().to_sexpr();

                Sexpr::List(vec![op, left, right])
            }
            Expr::Call(call) => {
                let mut items = vec![Sexpr::Atom("call".to_string()), call.target().to_sexpr()];

                for arg in call.args() {
                    items.push(arg.to_sexpr());
                }

                Sexpr::List(items)
            }
            Expr::Block(block) => block.to_sexpr(),
        }
    }
}

impl ToSexpr for Statement {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Statement::NoOp => Sexpr::List(vec![Sexpr::Atom("noop".to_string())]),
            Statement::Comment(text) => Sexpr::List(vec![
                Sexpr::Atom("comment".to_string()),
                Sexpr::Atom(format!("\"{}\"", text)),
            ]),
            Statement::Expr(expr) => expr.to_sexpr(),
            Statement::Assignment(name, expr) => Sexpr::List(vec![
                Sexpr::Atom("assign".to_string()),
                Sexpr::Atom(name.clone()),
                expr.to_sexpr(),
            ]),
            Statement::Field(name, type_expr, default) => {
                let mut items = vec![
                    Sexpr::Atom("field".to_string()),
                    Sexpr::Atom(name.clone()),
                    Sexpr::List(vec![Sexpr::Atom("type".to_string()), type_expr.to_sexpr()]),
                ];

                if let Some(default_expr) = default {
                    items.push(Sexpr::List(vec![
                        Sexpr::Atom("default".to_string()),
                        default_expr.to_sexpr(),
                    ]));
                }

                Sexpr::List(items)
            }
            Statement::Handler(handler) => handler.to_sexpr(),
        }
    }
}

impl ToSexpr for Block {
    fn to_sexpr(&self) -> Sexpr {
        let mut items = vec![Sexpr::Atom("block".to_string())];

        for stmt in self.statements() {
            items.push(stmt.to_sexpr());
        }

        Sexpr::List(items)
    }
}

impl ToSexpr for Pattern {
    fn to_sexpr(&self) -> Sexpr {
        let mut items = vec![Sexpr::Atom("pattern".to_string())];

        for term in self.terms() {
            items.push(term.to_sexpr());
        }

        Sexpr::List(items)
    }
}

impl ToSexpr for Handler {
    fn to_sexpr(&self) -> Sexpr {
        Sexpr::List(vec![
            Sexpr::Atom("handler".to_string()),
            self.pattern().to_sexpr(),
            self.block().to_sexpr(),
        ])
    }
}

impl ToSexpr for TypeExpr {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            TypeExpr::Empty => Sexpr::List(vec![Sexpr::Atom("empty".to_string())]),
            TypeExpr::Type(typ) => Sexpr::List(vec![
                Sexpr::Atom("type".to_string()),
                Sexpr::Atom(typ.to_string()),
            ]),
            TypeExpr::Word(w) => Sexpr::List(vec![
                Sexpr::Atom("word".to_string()),
                Sexpr::Atom(w.clone()),
            ]),
            TypeExpr::Value(value) => value.to_sexpr(),
            TypeExpr::Hole(hole) => Sexpr::List(vec![
                Sexpr::Atom("hole".to_string()),
                Sexpr::Atom(hole.clone()),
            ]),
            TypeExpr::BlockHole(block_hole) => Sexpr::List(vec![
                Sexpr::Atom("block-hole".to_string()),
                Sexpr::Atom(block_hole.clone()),
            ]),
            TypeExpr::TypeHole(name, typ) => Sexpr::List(vec![
                Sexpr::Atom("type-hole".to_string()),
                Sexpr::Atom(name.clone()),
                typ.to_sexpr(),
            ]),
        }
    }
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum ValueType {
//     User(String),
//     Empty,
//     Error,
//     Channel,
//     Boolean,
//     Word,
//     String,
//     Number,
//     List,
//     Block,
//     Bytes,
//     EmbeddedBlock,
// }

impl ToSexpr for ValueType {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            ValueType::User(name) => Sexpr::List(vec![
                Sexpr::Atom("user".to_string()),
                Sexpr::Atom(name.clone()),
            ]),
            ValueType::Empty => Sexpr::Atom("Empty".to_string()),
            ValueType::Error => Sexpr::Atom("Error".to_string()),
            ValueType::Channel => Sexpr::Atom("Channel".to_string()),
            ValueType::Boolean => Sexpr::Atom("Boolean".to_string()),
            ValueType::Word => Sexpr::Atom("Word".to_string()),
            ValueType::String => Sexpr::Atom("String".to_string()),
            ValueType::Number => Sexpr::Atom("Number".to_string()),
            ValueType::List => Sexpr::Atom("List".to_string()),
            ValueType::Block => Sexpr::Atom("Block".to_string()),
            ValueType::Bytes => Sexpr::Atom("Bytes".to_string()),
            ValueType::EmbeddedBlock => Sexpr::Atom("EmbeddedBlock".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Number;

    #[test]
    fn test_sexpr_format() {
        let sexpr = Sexpr::List(vec![
            Sexpr::Atom("foo".to_string()),
            Sexpr::List(vec![Sexpr::Atom("bar".to_string())]),
        ]);
        assert_eq!(sexpr.format(0), "(foo (bar))");
    }

    #[test]
    fn test_value_to_sexpr() {
        let value = Value::Number(Number::UInt(42));
        assert_eq!(value.to_sexpr().format(0), "(number 42)");
    }

    #[test]
    fn test_expr_to_sexpr() {
        let expr = Expr::Value(Value::Number(Number::UInt(42)));
        assert_eq!(expr.to_sexpr().format(0), "(number 42)");
    }
}
