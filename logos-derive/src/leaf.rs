use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug};

use syn::{Ident, Type, spanned::Spanned};
use proc_macro2::{TokenStream, Span};
use quote::quote;

use crate::graph::{Node, Disambiguate};

pub struct Leaf {
    pub ident: Ident,
    pub priority: usize,
    pub field: Option<Type>,
    pub callback: Callback,
}

#[derive(Debug)]
pub enum Callback {
    None,
    Label(TokenStream),
    Inline(Ident, TokenStream),
}

impl Callback {
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
            Some(ident) => Callback::Label(quote!(#ident)),
            None => Callback::None,
        }
    }
}

impl Leaf {
    pub fn token(ident: &Ident) -> Self {
        Leaf {
            ident: ident.clone(),
            priority: 0,
            field: None,
            callback: Callback::None,
        }
    }

    pub fn callback(mut self, callback: Callback) -> Self {
        self.callback = callback;
        self
    }

    pub fn field(mut self, field: Option<Type>) -> Self {
        self.field = field;
        self
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl Disambiguate for Leaf {
    fn cmp(left: &Leaf, right: &Leaf) -> Ordering {
        Ord::cmp(&left.priority, &right.priority)
    }
}

impl From<Leaf> for Node<Leaf> {
    fn from(leaf: Leaf) -> Self {
        Node::Leaf(leaf)
    }
}

impl Debug for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "::{}", self.ident)?;

        match self.callback {
            Callback::Label(ref label) => write!(f, " ({})", label),
            Callback::Inline(..) => f.write_str(" (<inline>)"),
            Callback::None => Ok(()),
        }
    }
}