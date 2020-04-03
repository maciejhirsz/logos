use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug};

use syn::Ident;
use proc_macro2::TokenStream;

use crate::graph::{Node, Disambiguate};

pub enum Leaf {
    Trivia,
    Token {
        ident: Ident,
        priority: usize,
        callback: Option<TokenStream>,
    },
}

impl Leaf {
    pub fn token(ident: &Ident) -> Self {
        Leaf::Token {
            ident: ident.clone(),
            priority: 0,
            callback: None,
        }
    }

    pub fn callback(mut self, cb: Option<TokenStream>) -> Self {
        match self {
            Leaf::Token { ref mut callback, .. } => *callback = cb,
            Leaf::Trivia => panic!("Oh no :("),
        }
        self
    }

    pub fn priority(mut self, prio: usize) -> Self {
        match self {
            Leaf::Token { ref mut priority, .. } => *priority = prio,
            Leaf::Trivia => panic!("Oh no :("),
        }
        self
    }
}

impl Disambiguate for Leaf {
    fn cmp(left: &Leaf, right: &Leaf) -> Ordering {
        match (left, right) {
            (Leaf::Token { priority: left, .. }, Leaf::Token { priority: right, .. }) => {
                Ord::cmp(left, right)
            },
            (Leaf::Token { .. }, Leaf::Trivia) => Ordering::Greater,
            (Leaf::Trivia, Leaf::Token { .. }) => Ordering::Less,
            (Leaf::Trivia, Leaf::Trivia) => Ordering::Equal,
        }
    }
}

impl From<Leaf> for Node<Leaf> {
    fn from(leaf: Leaf) -> Self {
        Node::Leaf(leaf)
    }
}

impl Debug for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Leaf::Trivia => f.write_str("<trivia>"),
            Leaf::Token { ident, callback, .. } => {
                 write!(f, "::{}", ident)?;

                if let Some(ref callback) = callback {
                    write!(f, " ({})", callback)?;
                }

                Ok(())
            }
        }
    }
}