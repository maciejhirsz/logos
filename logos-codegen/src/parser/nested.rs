use proc_macro2::{Ident, TokenStream};

use crate::parse::prelude::*;

pub struct Splitter {
    stream: ParseStream,
}

impl Splitter {
    pub fn new(stream: TokenStream) -> Self {
        Splitter {
            stream: stream.parse_stream(),
        }
    }
}

impl Iterator for Splitter {
    type Item = TokenStream;

    fn next(&mut self) -> Option<TokenStream> {
        let first = self.stream.next()?;

        if first.is(',') {
            return Some(TokenStream::new());
        }

        let mut out: TokenStream = first.into();

        out.extend((&mut self.stream).take_while(|tt| !tt.is(',')));

        Some(out)
    }
}

/// `name = ...`
pub struct NestedAssign<T = TokenStream> {
    pub value: T,
}

/// `name ident = ...`
pub struct NestedKeywordAssign<T = TokenStream> {
    pub name: Ident,
    pub value: T,
}

impl<T: Parse> Parse for NestedAssign<T> {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        stream.expect('=')?;

        Ok(NestedAssign {
            value: stream.parse()?,
        })
    }
}

impl<T: Parse> Parse for NestedKeywordAssign<T> {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        let name = stream.parse()?;

        stream.expect('=')?;

        Ok(NestedKeywordAssign {
            name,
            value: stream.parse()?,
        })
    }
}

pub struct Empty;

impl From<Empty> for TokenStream {
    fn from(_: Empty) -> TokenStream {
        TokenStream::new()
    }
}
