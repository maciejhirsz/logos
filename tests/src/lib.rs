//! ```compile_fail
//! use logos::Logos;
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
//! And also when working with bytes:
//!
//! ```compile_fail
//! use logos::Logos;
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
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     #[regex(b"\x00.+")]
//!     NonUtf8,
//!
//! }
//! ```
//!
//! Multiple export dirs should not compile.
//! When debug is not enabled, this also should not compile.
//!
//! ```compile_fail
//! use logos::Logos;
//!
//! #[derive(Logos)]
//! #[logos(export_dir = "target/tmp")]
//! #[logos(export_dir = "target/tmp")]
//! enum Token {}
//! ```
//!
//! A ".+" pattern shouldn't compile without `allow_greedy = true`
//!
//! ```compile_fail
//! use logos::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[regex("(a|b.*)")]
//!     Dotall,
//! }
//! ```
//!
//! A ".+" pattern should compile with `allow_greedy = true`
//!
//! ```
//! use logos::Logos;
//!
//! #[derive(Logos)]
//! enum Token {
//!     #[regex("(a|b.*)", allow_greedy = true)]
//!     Dotall,
//! }
//! ```
//!
//! https://github.com/maciejhirsz/logos/issues/232
//! This example fails because the subpattern can match the empty string,
//! ```compile_fail
//! use logos::Logos;
//!
//! #[derive(Logos)]
//! #[logos(subpattern example = r"(a|)+")]
//! enum Example1 {
//!     #[regex("(?&example)+")]
//!     Subpattern,
//! }
//!
//! #[derive(Logos)]
//! #[logos(subpattern example = r"(a|)+")]
//! enum Example2 {
//!     #[regex("(?&example)")]
//!     Subpattern,
//! }
//! ```
//!
//! This example fails because it has a priority conflict
//! ```compile_fail
//! use logos::Logos;
//!
//! #[derive(logos::Logos)]
//! enum Tokens {
//!     #[regex(r#"'(?:'?(?:[[:ascii:][^\\']]|\\[[:ascii:]]))*'"#)]
//!     #[regex(r#"'(?:"?(?:[[:ascii:][^\\"]]|\\[[:ascii:]]))*'"#)]
//!     Problem,
//! }

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
