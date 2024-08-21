use std::ops::Range;

use logos::{Logos as _, Source};
use logos_derive::Logos;

struct RefSource<'s, S: ?Sized + Source>(&'s S);

impl<'s, S: ?Sized + Source> Source for RefSource<'s, S> {
    type Slice<'a> = S::Slice<'a> where 's: 'a;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn read<'a, Chunk>(&'a self, offset: usize) -> Option<Chunk>
    where
        Chunk: logos::source::Chunk<'a>,
    {
        self.0.read(offset)
    }

    #[cfg(not(feature = "forbid_unsafe"))]
    unsafe fn read_byte_unchecked(&self, offset: usize) -> u8 {
        self.0.read_byte_unchecked(offset)
    }

    #[cfg(feature = "forbid_unsafe")]
    fn read_byte(&self, offset: usize) -> u8 {
        self.0.read_byte(offset)
    }

    fn slice(&self, range: Range<usize>) -> Option<Self::Slice<'_>> {
        self.0.slice(range)
    }

    #[cfg(not(feature = "forbid_unsafe"))]
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> Self::Slice<'_> {
        self.0.slice_unchecked(range)
    }

    fn is_boundary(&self, index: usize) -> bool {
        self.0.is_boundary(index)
    }
}

/// A simple regression test that it is possible to define a custom source.
///
/// Note that currently parenthesis are required around types with multiple
/// generic arguments.
#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(source = (RefSource<'s, str>))]
enum Token {
    #[regex(".")]
    Char,
}

#[test]
fn custom_source() {
    let source = RefSource("abc");
    let mut lex = Token::lexer(&source);

    assert_eq!(lex.next(), Some(Ok(Token::Char)));
    assert_eq!(lex.next(), Some(Ok(Token::Char)));
    assert_eq!(lex.next(), Some(Ok(Token::Char)));
    assert_eq!(lex.next(), None);
}
