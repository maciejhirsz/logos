use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Display};
use std::ops::Index;

use proc_macro2::{Span, TokenStream};
use regex_automata::PatternID;
use syn::{spanned::Spanned, Ident};

use crate::pattern::Pattern;
use crate::util::MaybeVoid;

#[derive(Clone)]
pub enum CallbackKind {
    Unit,
    Value(TokenStream),
    Skip,
}

#[derive(Clone)]
pub struct Leaf<'t> {
    pub pattern: Pattern,
    pub ident: Option<&'t Ident>,
    pub span: Span,
    pub priority: usize,
    pub kind: CallbackKind,
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

impl<'t> Leaf<'t> {
    pub fn new(ident: &'t Ident, span: Span, pattern: Pattern) -> Self {
        Leaf {
            pattern,
            ident: Some(ident),
            span,
            priority: 0,
            kind: CallbackKind::Unit,
            callback: None,
        }
    }

    pub fn new_skip(span: Span, pattern: Pattern) -> Self {
        Leaf {
            pattern,
            ident: None,
            span,
            priority: 0,
            kind: CallbackKind::Skip,
            callback: None,
        }
    }

    pub fn callback(mut self, callback: Option<Callback>) -> Self {
        self.callback = callback;
        self
    }

    pub fn field(mut self, field: MaybeVoid) -> Self {
        self.kind = match field {
            MaybeVoid::Some(field_ty) => CallbackKind::Value(field_ty),
            MaybeVoid::Void => CallbackKind::Unit,
        };
        self
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }

    pub fn compare_priority(left: &Self, right: &Self) -> Ordering {
        Ord::cmp(&left.priority, &right.priority)
    }
}

impl Debug for Leaf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "::{}", self)?;

        match self.callback {
            Some(Callback::Label(ref label)) => write!(f, " ({})", label),
            Some(Callback::Inline(_)) => f.write_str(" (<inline>)"),
            None => f.write_str(" (<none>)"),
        }
    }
}

impl Display for Leaf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.ident {
            Some(ident) => Display::fmt(ident, f),
            None => f.write_str("<skip>"),
        }
    }
}

/// Disambiguation error during the attempt to merge two leaf
/// nodes with the same priority
#[derive(Debug)]
pub struct DisambiguationError(pub LeafId, pub LeafId);

#[derive(Debug)]
pub struct Leaves<'a> {
    leaves: Vec<Leaf<'a>>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeafId(pub u32);

impl From<usize> for LeafId {
    fn from(value: usize) -> Self {
        LeafId(value.try_into().expect("More than 2^32 nodes"))
    }
}

impl From<PatternID> for LeafId {
    fn from(value: PatternID) -> Self {
        value.as_usize().into()
    }
}

impl<'a> Leaves<'a> {
    pub fn new() -> Self {
        Leaves { leaves: Vec::new() }
    }

    pub fn push(&mut self, leaf: Leaf<'a>)
    {
        let idx = match self.leaves
            .binary_search_by_key(&leaf.priority, |leaf| leaf.priority)
        {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        self.leaves.insert(idx, leaf);
    }

    pub fn errors(&self) -> Vec<DisambiguationError> {
        let mut errors = Vec::new();
        for i in 0..self.leaves.len()-1 {
            if self.leaves[i].priority == self.leaves[i+1].priority {
                errors.push(DisambiguationError(i.into(), (i + 1).into()));
            }
        }

        errors
    }

    pub fn iter(&self) -> impl Iterator<Item=&Leaf<'a>> {
        self.leaves.iter()
    }
}

impl<'a> Index<LeafId> for Leaves<'a> {
    type Output = Leaf<'a>;
    fn index(&self, index: LeafId) -> &Self::Output {
        &self.leaves[index.0 as usize]
    }
}

impl<'a> From<Leaves<'a>> for Vec<Leaf<'a>> {
    fn from(value: Leaves<'a>) -> Self {
        value.leaves
    }
}
