use proc_macro2::TokenStream;
use quote::quote;

use crate::graph::NodeId;
use crate::generator::Generator;

/// This struct keeps track of bytes available to be read without
/// bounds checking across the tree.
///
/// For example, a branch that matches 4 bytes followed by a fork
/// with smallest branch containing of 2 bytes can do a bounds check
/// for 6 bytes ahead, and leave the remaining 2 byte array (fixed size)
/// to be handled by the fork, avoiding bound checks there.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Context {
    /// Amount of bytes that haven't been bumped yet but should
    /// before a new read is performed
    at: usize,
    /// Number of bytes available without bound checks
    available: usize,
    /// Whether or not the Lexer has been bumped at least by 1 byte
    bumped: bool,
    /// Node to backtrack to to in case an explicit match has failed.
    /// If `None` will instead produce an error token.
    fallback: Option<NodeId>,
}

impl Context {
    const fn backtrack(self) -> Self {
        Context {
            at: 0,
            available: 0,
            bumped: self.bumped,
            fallback: None,
        }
    }

    pub fn has_fallback(&self) -> bool {
        self.fallback.is_some()
    }

    pub fn switch(&mut self, miss: Option<NodeId>) -> Option<TokenStream> {
        if let Some(miss) = miss {
            self.fallback = Some(miss);
        }
        self.bump()
    }

    pub const fn advance(self, n: usize) -> Self {
        Context {
            at: self.at + n,
            ..self
        }
    }

    pub fn bump(&mut self) -> Option<TokenStream> {
        match self.at {
            0 => None,
            n => {
                let tokens = quote!(lex.bump(#n););
                self.at = 0;
                self.bumped = true;
                Some(tokens)
            },
        }
    }

    pub fn read(&self, len: usize) -> TokenStream {
        match (self.at, len) {
            (0, 0) => quote!(lex.read::<u8>()),
            (a, 0) => quote!(lex.read_at::<u8>(#a)),
            (0, l) => quote!(lex.read::<&[u8; #l]>()),
            (a, l) => quote!(lex.read_at::<&[u8; #l]>(#a)),
        }
    }

    pub fn miss(self, miss: Option<NodeId>, gen: &mut Generator) -> TokenStream {
        match (miss, self.fallback) {
            (Some(id), _) => gen.goto(id, self).clone(),
            (_, Some(id)) => gen.goto(id, self.backtrack()).clone(),
            _ if self.bumped => quote!(lex.error()),
            _ => quote!(_error(lex)),
        }
    }

    pub fn write_suffix(&self, buf: &mut String) {
        use std::fmt::Write;

        if self.at > 0 {
            let _ = write!(buf, "_at{}", self.at);
        }
        if let Some(id) = self.fallback {
            let _ = write!(buf, "_ctx{}", id);
        }
        if self.bumped {
            buf.push_str("_x");
        }
    }
}