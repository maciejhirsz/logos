use std::fmt;

use super::Node;

pub type Token<'a> = &'a ::syn::Ident;
pub type Callback = ::syn::Ident;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Leaf<'a> {
    pub token: Token<'a>,
    pub callback: Option<Callback>,
}

impl<'a> Leaf<'a> {
    pub fn take(&mut self) -> Leaf<'a> {
        Leaf {
            token: self.token,
            callback: self.callback.take(),
        }
    }
}

impl<'a> fmt::Debug for Leaf<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.token)?;

        if let Some(ref callback) = self.callback {
            write!(f, " ({})", callback)?;
        }

        Ok(())
    }
}

impl<'a> From<Token<'a>> for Leaf<'a> {
    fn from(token: Token<'a>) -> Self {
        Leaf {
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
