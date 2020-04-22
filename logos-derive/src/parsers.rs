use proc_macro2::token_stream::IntoIter as TokenIter;
use proc_macro2::{Ident, TokenTree, TokenStream};

use crate::util::is_punct;

pub enum NestedValue {
    /// `name`
    None,
    /// `name = ...`
    Assign(TokenStream),
    /// `name(...)`
    Group(TokenStream),
    /// `name ident = ...`
    KeywordAssign(Ident, TokenStream),
}

pub enum Nested {
    /// Unnamed nested attribute, such as a string,
    /// callback closure, or a lone path
    ///
    /// Note: a lone ident will be Named with no value instead
    Unnamed(TokenStream),
    /// Named: name ...
    Named(Ident, NestedValue),
    /// Unexpected token,
    Unexpected(TokenStream),
}

pub struct AttributeParser {
    inner: TokenIter,
}

pub struct Empty;

impl From<Empty> for TokenStream {
    fn from(_: Empty) -> TokenStream {
        TokenStream::new()
    }
}

impl AttributeParser {
    pub fn new(stream: TokenStream) -> Self {
        AttributeParser {
            inner: stream.into_iter()
        }
    }

    fn next_tt(&mut self) -> Option<TokenTree> {
        match self.inner.next() {
            Some(tt) if is_punct(&tt, ',') => None,
            next => next,
        }
    }

    fn collect_tail<T>(&mut self, first: T) -> TokenStream
    where
        T: Into<TokenStream>,
    {
        let mut out = first.into();

        while let Some(tt) = self.next_tt() {
            out.extend(Some(tt));
        }

        out
    }

    fn parse_unnamed(&mut self, first: Ident, next: TokenTree) -> Nested {
        let mut out = TokenStream::from(TokenTree::Ident(first));

        out.extend(self.collect_tail(next));

        Nested::Unnamed(out)
    }

    fn parse_assign(&mut self, name: Ident) -> Nested {
        let value = self.collect_tail(Empty);

        Nested::Named(name, NestedValue::Assign(value))
    }

    fn parse_group(&mut self, name: Ident, group: TokenStream) -> Nested {
        let error = self.collect_tail(Empty);

        if error.is_empty() {
            Nested::Named(name, NestedValue::Group(group))
        } else {
            Nested::Unexpected(error.into())
        }
    }

    fn parse_keyword(&mut self, keyword: Ident, name: Ident) -> Nested {
        let error = match self.next_tt() {
            Some(tt) if is_punct(&tt, '=') => None,
            next => next,
        };

        match error {
            Some(error) => {
                let error = self.collect_tail(error);

                Nested::Unexpected(error)
            },
            None => {
                let value = self.collect_tail(Empty);

                Nested::Named(keyword, NestedValue::KeywordAssign(name, value))
            },
        }
    }
}

impl Iterator for AttributeParser {
    type Item = Nested;

    fn next(&mut self) -> Option<Nested> {
        let first = self.inner.next()?;

        let name = match first {
            TokenTree::Ident(ident) => ident,
            tt => {
                let stream = self.collect_tail(tt);

                return Some(Nested::Unnamed(stream));
            }
        };

        match self.next_tt() {
            Some(tt) if is_punct(&tt, '=') => Some(self.parse_assign(name)),
            Some(TokenTree::Group(group)) => Some(self.parse_group(name, group.stream())),
            Some(TokenTree::Ident(next)) => Some(self.parse_keyword(name, next)),
            Some(next) => Some(self.parse_unnamed(name, next)),
            None => Some(Nested::Named(name, NestedValue::None)),
        }
    }
}