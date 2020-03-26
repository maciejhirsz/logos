use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug};

use syn::Ident;

use crate::graph::{Node, Disambiguate};

#[cfg_attr(test, derive(PartialEq))]
pub struct Token {
    pub ident: Ident,
    pub priority: usize,
    pub callback: Option<Ident>,
}

impl Token {
    pub fn new(ident: &Ident) -> Self {
        Token {
            ident: ident.clone(),
            priority: 0,
            callback: None,
        }
    }

    pub fn callback(mut self, callback: Option<Ident>) -> Self {
        self.callback = callback;
        self
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl Disambiguate for Token {
    fn cmp(left: &Token, right: &Token) -> Ordering {
        Ord::cmp(&left.priority, &right.priority)
    }
}

impl From<Token> for Node<Token> {
    fn from(leaf: Token) -> Self {
        Node::Leaf(leaf)
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "::{}", self.ident)?;

        if let Some(ref callback) = self.callback {
            write!(f, " ({})", callback)?;
        }

        Ok(())
    }
}