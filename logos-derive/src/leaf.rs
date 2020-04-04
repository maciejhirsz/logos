use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug};

use syn::Ident;
use proc_macro2::{TokenStream, Span};

use crate::graph::{Node, Disambiguate};

pub enum Leaf {
    Trivia,
    Token {
        ident: Ident,
        priority: usize,
        callback: Callback,
    },
}

pub enum Callback {
    None,
    Label(Ident),
    Inline(Ident, TokenStream),
}

impl Callback {
    pub fn or_else<F>(self, f: F) -> Callback
    where
        F: Fn() -> Option<Ident>,
    {
        match self {
            Callback::None => f().into(),
            _ => self,
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Callback::Label(ident) => Some(ident.span()),
            Callback::Inline(arg, ..) => Some(arg.span()),
            _ => None,
        }
    }
}

impl From<Option<Ident>> for Callback {
    fn from(label: Option<Ident>) -> Self {
        match label {
            Some(ident) => Callback::Label(ident),
            None => Callback::None,
        }
    }
}

impl Leaf {
    pub fn token(ident: &Ident) -> Self {
        Leaf::Token {
            ident: ident.clone(),
            priority: 0,
            callback: Callback::None,
        }
    }

    pub fn callback(mut self, cb: Callback) -> Self {
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

                match callback {
                    Callback::Label(ref label) => write!(f, " ({})", label)?,
                    Callback::Inline(..) => f.write_str(" (<inline>)")?,
                    _ => (),
                }

                Ok(())
            }
        }
    }
}