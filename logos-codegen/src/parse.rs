//! [`ParseStream`](ParseStream), the [`Parse`](Parse) trait and utilities for working with
//! token streams without `syn` or `quote`.

use beef::lean::Cow;
use proc_macro2::{Delimiter, Ident, Literal, Spacing, Span, TokenStream, TokenTree};

pub type ParseStream = std::iter::Peekable<proc_macro2::token_stream::IntoIter>;

pub mod prelude {
    pub use super::{IdentExt, IteratorExt, TokenStreamExt, TokenTreeExt};
    pub use super::{IntoSpan, Lit, Parse, ParseError, ParseStream};
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: Cow<'static, str>,
    pub span: Span,
}

impl ParseError {
    pub fn explain(mut self, explanation: &str) -> Self {
        self.msg = {
            let mut msg = self.msg.into_owned();

            msg.push_str("\n\n");
            msg.push_str(explanation);

            msg.into()
        };

        self
    }
}

pub trait IntoSpan {
    fn into_span(self) -> Span;
}

impl IntoSpan for Span {
    fn into_span(self) -> Span {
        self
    }
}

impl IntoSpan for TokenTree {
    fn into_span(self) -> Span {
        self.span()
    }
}

impl IntoSpan for TokenStream {
    fn into_span(self) -> Span {
        self.into_iter().next().into_span()
    }
}

impl IntoSpan for Option<TokenTree> {
    fn into_span(self) -> Span {
        self.as_ref()
            .map(TokenTree::span)
            .unwrap_or_else(Span::call_site)
    }
}

impl ParseError {
    pub fn new<M, S>(msg: M, spannable: S) -> Self
    where
        M: Into<Cow<'static, str>>,
        S: IntoSpan,
    {
        ParseError {
            msg: msg.into(),
            span: spannable.into_span(),
        }
    }
}

pub trait Pattern: Copy {
    fn matches(self, tt: &TokenTree) -> bool;

    fn expected(self) -> Cow<'static, str>;
}

pub trait Parse: Sized {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError>;
}

impl Parse for TokenStream {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        Ok(stream.collect())
    }
}

impl Parse for Ident {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        match stream.next() {
            Some(TokenTree::Ident(ident)) => Ok(ident),
            tt => Err(ParseError::new("Expected an identifier", tt)),
        }
    }
}

impl Parse for Literal {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        match stream.next() {
            Some(TokenTree::Literal(lit)) => Ok(lit),
            tt => Err(ParseError::new("Expected a literal value", tt)),
        }
    }
}

impl Parse for () {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        match stream.next() {
            Some(tt) => Err(ParseError::new("Unexpected token", tt)),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Lit;

impl Pattern for Lit {
    fn matches(self, tt: &TokenTree) -> bool {
        matches!(tt, TokenTree::Literal(_))
    }

    fn expected(self) -> Cow<'static, str> {
        "Expected a literal value".into()
    }
}

impl Pattern for &str {
    fn matches(self, tt: &TokenTree) -> bool {
        match tt {
            TokenTree::Ident(ident) => ident.eq_str(self),
            _ => false,
        }
    }

    fn expected(self) -> Cow<'static, str> {
        format!("Expected {self}").into()
    }
}

impl Pattern for char {
    fn matches(self, tt: &TokenTree) -> bool {
        let delimiter = match self {
            '{' => Some(Delimiter::Brace),
            '[' => Some(Delimiter::Bracket),
            '(' => Some(Delimiter::Parenthesis),
            _ => None,
        };

        if let Some(delimiter) = delimiter {
            return delimiter.matches(tt);
        }

        match tt {
            TokenTree::Punct(punct) => punct.as_char() == self,
            _ => false,
        }
    }

    fn expected(self) -> Cow<'static, str> {
        format!("Expected {self}").into()
    }
}

impl Pattern for (char, Spacing) {
    fn matches(self, tt: &TokenTree) -> bool {
        match tt {
            TokenTree::Punct(punct) => punct.as_char() == self.0 && punct.spacing() == self.1,
            _ => false,
        }
    }

    fn expected(self) -> Cow<'static, str> {
        format!("Expected {}", self.0).into()
    }
}

impl Pattern for Delimiter {
    fn matches(self, tt: &TokenTree) -> bool {
        match tt {
            TokenTree::Group(group) => group.delimiter() == self,
            _ => false,
        }
    }

    fn expected(self) -> Cow<'static, str> {
        match self {
            Delimiter::Parenthesis => "Expected (...)",
            Delimiter::Brace => "Expected {...}",
            Delimiter::Bracket => "Expected [...]",
            Delimiter::None => "Expected a group",
        }
        .into()
    }
}

pub trait IteratorExt {
    fn expect(&mut self, pattern: impl Pattern) -> Result<TokenTree, ParseError>;

    fn allow(&mut self, pattern: impl Pattern) -> bool;

    fn allow_consume(&mut self, pattern: impl Pattern) -> Option<TokenTree>;

    fn parse<T: Parse>(&mut self) -> Result<T, ParseError>;

    fn end(&mut self) -> bool;
}

impl IteratorExt for ParseStream {
    fn expect(&mut self, pattern: impl Pattern) -> Result<TokenTree, ParseError> {
        match self.next() {
            Some(tt) if pattern.matches(&tt) => Ok(tt),
            tt => Err(ParseError::new(pattern.expected(), tt)),
        }
    }

    fn allow(&mut self, pattern: impl Pattern) -> bool {
        self.peek().map(|tt| pattern.matches(tt)).unwrap_or(false)
    }

    fn allow_consume(&mut self, pattern: impl Pattern) -> Option<TokenTree> {
        self.next_if(|tt| pattern.matches(tt))
    }

    fn parse<T: Parse>(&mut self) -> Result<T, ParseError> {
        T::parse(self)
    }

    fn end(&mut self) -> bool {
        self.peek().is_none()
    }
}

pub trait TokenStreamExt: Sized {
    fn parse_stream(self) -> ParseStream;

    fn parse<T: Parse>(self) -> Result<T, ParseError> {
        let mut stream = self.parse_stream();

        let out = stream.parse()?;

        stream.parse::<()>()?;

        Ok(out)
    }
}

impl TokenStreamExt for TokenStream {
    fn parse_stream(self) -> ParseStream {
        self.into_iter().peekable()
    }
}

pub trait TokenTreeExt {
    fn is(&self, pattern: impl Pattern) -> bool;
}

impl TokenTreeExt for TokenTree {
    fn is(&self, pattern: impl Pattern) -> bool {
        pattern.matches(self)
    }
}

impl TokenTreeExt for Option<TokenTree> {
    fn is(&self, pattern: impl Pattern) -> bool {
        self.as_ref().map(|tt| pattern.matches(tt)).unwrap_or(false)
    }
}

mod util {
    use std::cell::RefCell;
    use std::fmt::{Display, Write};

    use arrayvec::ArrayString;

    thread_local! {
        static FMT_BUF: RefCell<ArrayString<40>> = RefCell::new(ArrayString::new());
    }

    pub trait IdentExt: Display {
        fn with_str<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&str) -> R,
        {
            FMT_BUF.with(move |buf| {
                let buf = buf.try_borrow_mut().ok().and_then(|mut buf| {
                    buf.clear();

                    write!(buf, "{self}").ok()?;

                    Some(buf)
                });

                match buf {
                    Some(buf) => f(&buf),
                    None => f(&self.to_string()),
                }
            })
        }

        fn eq_str(&self, other: &str) -> bool {
            self.with_str(|s| s == other)
        }

        fn one_of<'a>(&self, other: impl IntoIterator<Item = &'a str>) -> bool {
            self.with_str(move |s| other.into_iter().any(|other| s == other))
        }
    }

    impl IdentExt for proc_macro2::Ident {}
}

pub use util::IdentExt;
