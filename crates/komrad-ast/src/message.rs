use crate::channel::Channel;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Message {
    terms: Vec<Value>,
    reply_to: Option<Channel>,
}

impl Message {
    pub fn new(terms: Vec<Value>, reply_to: Option<Channel>) -> Self {
        Message { terms, reply_to }
    }

    pub fn default() -> Self {
        Message {
            terms: Vec::new(),
            reply_to: None,
        }
    }

    pub fn terms(&self) -> &Vec<Value> {
        &self.terms
    }

    pub fn reply_to(&self) -> Option<Channel> {
        self.reply_to.clone()
    }
}

pub trait MessageBuilder {
    fn with_terms(self, terms: Vec<Value>) -> Self;
    fn with_reply_to(self, reply_to: Option<Channel>) -> Self;

    fn with_term(self, term: Value) -> Self;
}

impl MessageBuilder for Message {
    fn with_terms(mut self, terms: Vec<Value>) -> Self {
        self.terms.extend(terms);
        self
    }

    fn with_reply_to(mut self, reply_to: Option<Channel>) -> Self {
        self.reply_to = reply_to;
        self
    }

    fn with_term(mut self, term: Value) -> Self {
        self.terms.push(term);
        self
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
