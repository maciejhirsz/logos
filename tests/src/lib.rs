//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[token(b"\xFF")]
//!     NonUtf8,
//! }
//!
//! Token::lexer("This shouldn't work with a string literal!");
//! ```
//!
//! Same, but with regex:
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[regex(b"\xFF")]
//!     NonUtf8,
//! }
//!
//! Token::lexer("This shouldn't work with a string literal!");
//! ```
//!
//! Matching against .* (or .+) should fail to compile:
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     #[regex(r"\(.*\)")]
//!     BetweenParen,
//!
//! }
//! ```
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     #[regex(r"\(.+\)")]
//!     BetweenParen,
//!
//! }
//! ```
//!
//! And also when working with bytes:
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     #[regex(b"\x00.*")]
//!     NonUtf8,
//!
//! }
//! ```
//!
//! ```compile_fail
//! use logos::Logos;
//! use logos_derive::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     #[regex(b"\x00.+")]
//!     NonUtf8,
//!
//! }
//! ```

use logos::source::Source;
use logos::Logos;

use std::fmt;
use std::ops::Range;

#[allow(clippy::type_complexity)]
pub fn assert_lex<'a, Token>(
    source: &'a Token::Source,
    tokens: &[(
        Result<Token, Token::Error>,
        <Token::Source as Source>::Slice<'a>,
        Range<usize>,
    )],
) where
    Token: Logos<'a> + fmt::Debug + PartialEq,
    Token::Extras: Default,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(
            &(lex.next().expect("Unexpected end"), lex.slice(), lex.span()),
            tuple
        );
    }

    assert_eq!(lex.next(), None);
}
