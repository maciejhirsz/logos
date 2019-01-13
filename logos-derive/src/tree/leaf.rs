use std::{fmt, mem};

use super::Node;

pub type Token<'a> = &'a ::syn::Ident;
pub type Callback = ::syn::Ident;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Leaf<'a> {
    Token {
        token: Token<'a>,
        callback: Option<Callback>,
    },
    Trivia,
}

impl<'a> Leaf<'a> {
    pub fn take(&mut self) -> Leaf<'a> {
        mem::replace(self, Leaf::Trivia)
    }
}

impl<'a> fmt::Debug for Leaf<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Leaf::Token { token, callback } => {
                write!(f, "{}", token)?;

                if let Some(ref callback) = callback {
                    write!(f, " ({})", callback)?;
                }
            },
            Leaf::Trivia => write!(f, "TRIVIA")?,
        }

        Ok(())
    }
}

impl<'a> From<Token<'a>> for Leaf<'a> {
    fn from(token: Token<'a>) -> Self {
        Leaf::Token {
            token,
            callback: None,
        }
    }
}

impl<'a> From<Token<'a>> for Node<'a> {
    fn from(token: Token<'a>) -> Self {
        Node::Leaf(token.into())
    }
}

impl<'a> From<Leaf<'a>> for Node<'a> {
    fn from(leaf: Leaf<'a>) -> Self {
        Node::Leaf(leaf)
    }
}
