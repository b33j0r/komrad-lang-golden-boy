use crate::operators::{BinaryOp, UnaryOp};
use crate::pattern::Pattern;
use crate::types::{Block, Expr, Handler, Msg, Statement, Value};
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

impl ToSexpr for Value {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Value::Bool(b) => {
                Sexpr::List(vec![Sexpr::Atom("bool".into()), Sexpr::Atom(b.to_string())])
            }
            Value::String(s) => Sexpr::List(vec![
                Sexpr::Atom("string".into()),
                Sexpr::Atom(format!("\"{}\"", s)),
            ]),
            Value::Int(n) => {
                Sexpr::List(vec![Sexpr::Atom("int".into()), Sexpr::Atom(n.to_string())])
            }
            Value::UInt(n) => {
                Sexpr::List(vec![Sexpr::Atom("uint".into()), Sexpr::Atom(n.to_string())])
            }
            Value::Float(n) => Sexpr::List(vec![
                Sexpr::Atom("float".into()),
                Sexpr::Atom(n.to_string()),
            ]),
            Value::Bytes(b) => Sexpr::List(vec![
                Sexpr::Atom("bytes".into()),
                Sexpr::Atom(format!("{:?}", b)),
            ]),
            Value::Word(w) => Sexpr::Atom(w.clone()),
            Value::List(l) => Sexpr::List(
                std::iter::once(Sexpr::Atom("list".into()))
                    .chain(l.iter().map(|v| v.to_sexpr()))
                    .collect(),
            ),
            Value::Msg(m) => m.to_sexpr(),
            _ => Sexpr::Atom(format!("{:?}", self)),
        }
    }
}

impl ToSexpr for Msg {
    fn to_sexpr(&self) -> Sexpr {
        let mut list = vec![Sexpr::Atom("msg".into()), self.callee.to_sexpr()];
        if let Some(command) = &self.command {
            list.push(Sexpr::Atom(command.clone()));
        }
        list.extend(self.message.iter().map(|m| m.to_sexpr()));
        if let Some(reply) = &self.reply {
            list.push(Sexpr::List(vec![
                Sexpr::Atom("reply".into()),
                reply.to_sexpr(),
            ]));
        }
        Sexpr::List(list)
    }
}

impl ToSexpr for Expr {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Expr::Value(v) => v.to_sexpr(),
            Expr::Variable(name) => {
                Sexpr::List(vec![Sexpr::Atom("var".into()), Sexpr::Atom(name.clone())])
            }
            Expr::Unary { op, expr } => Sexpr::List(vec![
                Sexpr::Atom("unary".into()),
                Sexpr::Atom(format!("{:?}", op)),
                expr.to_sexpr(),
            ]),
            Expr::Binary { left, op, right } => Sexpr::List(vec![
                Sexpr::Atom("binary".into()),
                Sexpr::Atom(format!("{:?}", op)),
                left.to_sexpr(),
                right.to_sexpr(),
            ]),
            Expr::Command {
                callee,
                command,
                args,
            } => {
                let mut list = vec![
                    Sexpr::Atom("command".into()),
                    callee.to_sexpr(),
                    Sexpr::Atom(command.clone()),
                ];
                list.extend(args.iter().map(|a| a.to_sexpr()));
                Sexpr::List(list)
            }
            Expr::Tell { callee, args } => {
                let mut list = vec![Sexpr::Atom("tell".into()), callee.to_sexpr()];
                list.extend(args.iter().map(|a| a.to_sexpr()));
                Sexpr::List(list)
            }
            Expr::Ask {
                callee,
                args,
                reply,
            } => {
                let mut list = vec![Sexpr::Atom("ask".into()), callee.to_sexpr()];
                list.extend(args.iter().map(|a| a.to_sexpr()));
                list.push(Sexpr::List(vec![
                    Sexpr::Atom("reply".into()),
                    reply.to_sexpr(),
                ]));
                Sexpr::List(list)
            }
        }
    }
}

impl ToSexpr for Statement {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            Statement::Expr(expr) => Sexpr::List(vec![Sexpr::Atom("stmt".into()), expr.to_sexpr()]),
            Statement::Assignment(name, expr) => Sexpr::List(vec![
                Sexpr::Atom("assign".into()),
                Sexpr::Atom(name.clone()),
                expr.to_sexpr(),
            ]),
            Statement::NoOp => Sexpr::Skip,
            Statement::Comment(s) => Sexpr::List(vec![
                Sexpr::Atom("comment".into()),
                Sexpr::Atom(format!("\"{}\"", s)),
            ]),
            Statement::Handler(handler) => handler.to_sexpr(),
            Statement::Field(field) => {
                let mut list = vec![
                    Sexpr::Atom("field".into()),
                    Sexpr::Atom(field.name.clone()),
                    Sexpr::Atom(field.typ.clone()),
                ];
                if let Some(expr) = &field.expr {
                    list.push(expr.to_sexpr());
                }
                Sexpr::List(list)
            }
        }
    }
}

impl ToSexpr for Block {
    fn to_sexpr(&self) -> Sexpr {
        let mut list = vec![Sexpr::Atom("block".into())];
        for stmt in &self.statements {
            list.push(stmt.to_sexpr());
        }
        Sexpr::List(list)
    }
}

impl ToSexpr for Handler {
    fn to_sexpr(&self) -> Sexpr {
        Sexpr::List(vec![
            Sexpr::Atom("handler".into()),
            Sexpr::Atom(self.name.clone()),
            self.pattern.to_sexpr(),
            self.block.to_sexpr(),
        ])
    }
}

impl ToSexpr for Pattern {
    fn to_sexpr(&self) -> Sexpr {
        let mut list = vec![Sexpr::Atom("pattern".into())];
        for term in self.terms().iter() {
            list.push(term.to_sexpr());
        }
        Sexpr::List(list)
    }
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
            BinaryOp::Mod => Sexpr::Atom("%".into()),
            BinaryOp::And => Sexpr::Atom("&&".into()),
            BinaryOp::Or => Sexpr::Atom("||".into()),
            BinaryOp::Xor => Sexpr::Atom("^".into()),
            BinaryOp::Shl => Sexpr::Atom("<<".into()),
            BinaryOp::Shr => Sexpr::Atom(">>".into()),
        }
    }
}
