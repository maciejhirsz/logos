use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Display};

use proc_macro2::{Span, TokenStream};
use regex_automata::PatternID;
use syn::{spanned::Spanned, Ident};

use crate::pattern::Pattern;
use crate::util::MaybeVoid;

#[derive(Clone)]
pub enum VariantKind {
    Unit(Ident),
    Value(Ident, TokenStream),
    Skip,
}

#[derive(Clone)]
pub struct Leaf {
    pub pattern: Pattern,
    pub span: Span,
    pub priority: usize,
    pub kind: VariantKind,
    pub callback: Option<Callback>,
}

#[derive(Clone)]
pub enum Callback {
    Label(TokenStream),
    Inline(Box<InlineCallback>),
}

#[derive(Clone)]
pub struct InlineCallback {
    pub arg: Ident,
    pub body: TokenStream,
    pub span: Span,
}

impl From<InlineCallback> for Callback {
    fn from(inline: InlineCallback) -> Callback {
        Callback::Inline(Box::new(inline))
    }
}

impl Callback {
    pub fn span(&self) -> Span {
        match self {
            Callback::Label(tokens) => tokens.span(),
            Callback::Inline(inline) => inline.span,
        }
    }
}

impl Leaf {
    pub fn new(span: Span, pattern: Pattern) -> Self {
        Leaf {
            pattern,
            span,
            priority: 0,
            kind: VariantKind::Skip,
            callback: None,
        }
    }

    pub fn variant_kind(self, kind: VariantKind) -> Self {
        Self { kind, ..self }
    }

    pub fn callback(self, callback: Option<Callback>) -> Self {
        Self { callback, ..self }
    }

    pub fn priority(self, priority: usize) -> Self {
        Self { priority, ..self }
    }

    pub fn compare_priority(left: &Self, right: &Self) -> Ordering {
        Ord::cmp(&left.priority, &right.priority)
    }
}

impl Debug for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "::{}", self)?;

        match self.callback {
            Some(Callback::Label(ref label)) => write!(f, " ({})", label),
            Some(Callback::Inline(_)) => f.write_str(" (<inline>)"),
            None => f.write_str(" (<none>)"),
        }
    }
}

impl Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            VariantKind::Unit(ident) | VariantKind::Value(ident, _) => Display::fmt(ident, f),
            VariantKind::Skip => f.write_str("<skip>"),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeafId(pub usize);

impl Default for LeafId {
    fn default() -> Self {
        LeafId(0)
    }
}

impl From<PatternID> for LeafId {
    fn from(value: PatternID) -> Self {
        LeafId(value.as_usize())
    }
}

