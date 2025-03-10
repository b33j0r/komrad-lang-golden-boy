use crate::ast::Value;
use crate::channel::Channel;

#[derive(Debug, Clone)]
pub struct Message {
    terms: Vec<Value>,
    reply_to: Option<Channel>,
}

impl Message {
    pub fn new(terms: Vec<Value>, reply_to: Option<Channel>) -> Self {
        Message { terms, reply_to }
    }

    pub fn terms(&self) -> &Vec<Value> {
        &self.terms
    }

    pub fn reply_to(&self) -> Option<Channel> {
        self.reply_to.clone()
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.terms == other.terms && self.reply_to == other.reply_to
    }
}

impl Message {
    pub fn first_word(&self) -> Option<String> {
        if let Some(first_term) = self.terms.get(0) {
            if let Value::Word(name) = first_term {
                return Some(name.clone());
            }
        }
        None
    }

    pub fn rest(&self) -> &[Value] {
        if self.terms.is_empty() {
            return &[];
        }
        &self.terms[1..]
    }
}
