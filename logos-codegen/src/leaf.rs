use std::cmp::Ord;
use std::fmt::{self, Debug, Display};

use proc_macro2::{Span, TokenStream};
use regex_automata::PatternID;
use syn::{spanned::Spanned, Ident};

use crate::pattern::Pattern;

#[derive(Clone, Debug)]
pub enum VariantKind {
    Unit(Ident),
    Value(Ident, TokenStream),
    Skip,
}

#[derive(Debug, Clone)]
pub struct Leaf {
    pub pattern: Pattern,
    pub span: Span,
    pub priority: usize,
    pub kind: VariantKind,
    pub callback: Option<Callback>,
}

#[derive(Clone, Debug)]
pub enum Callback {
    Label(TokenStream),
    Inline(Box<InlineCallback>),
}

#[derive(Clone, Debug)]
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
}

impl Display for VariantKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            VariantKind::Unit(ident)  => write!(f, "::{ident}"),
            VariantKind::Value(ident, _) => write!(f, "::{ident}(_)"),
            VariantKind::Skip => f.write_str("::<skip>"),
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
