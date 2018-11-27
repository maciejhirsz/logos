//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! **Logos** works by:
//! + Resolving all logical branching of token definitions into a tree.
//! + Optimizing complex patterns into [Lookup Tables](https://en.wikipedia.org/wiki/Lookup_table).
//! + Always using a Lookup Table for the first byte of a token.
//! + Producing code that never backtracks, thus running at linear time or close to it.
//!
//! In practice it means that for most grammars the lexing performance is virtually unaffected by the number
//! of tokens defined in the grammar. Or, in other words, **it is really fast**.
//!
//! ## Example
//!
//! ```rust
//! extern crate logos;
//!
//! use logos::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     // Logos requires that we define two default variants,
//!     // one for end of input source,
//!     #[end]
//!     End,
//!
//!     // ...and one for errors. Those can be named anything
//!     // you wish as long as the attributes are there.
//!     #[error]
//!     Error,
//!
//!     // Tokens can be literal strings, of any length.
//!     #[token = "fast"]
//!     Fast,
//!
//!     #[token = "."]
//!     Period,
//!
//!     // Or regular expressions.
//!     #[regex = "[a-zA-Z]+"]
//!     Text,
//! }
//!
//! fn main() {
//!     let mut lexer = Token::lexer("Create ridiculously fast Lexers.");
//!
//!     assert_eq!(lexer.token, Token::Text);
//!     assert_eq!(lexer.slice(), "Create");
//!     assert_eq!(lexer.range(), 0..6);
//!
//!     lexer.advance();
//!
//!     assert_eq!(lexer.token, Token::Text);
//!     assert_eq!(lexer.slice(), "ridiculously");
//!     assert_eq!(lexer.range(), 7..19);
//!
//!     lexer.advance();
//!
//!     assert_eq!(lexer.token, Token::Fast);
//!     assert_eq!(lexer.slice(), "fast");
//!     assert_eq!(lexer.range(), 20..24);
//!
//!     lexer.advance();
//!
//!     assert_eq!(lexer.token, Token::Text);
//!     assert_eq!(lexer.slice(), "Lexers");
//!     assert_eq!(lexer.range(), 25..31);
//!
//!     lexer.advance();
//!
//!     assert_eq!(lexer.token, Token::Period);
//!     assert_eq!(lexer.slice(), ".");
//!     assert_eq!(lexer.range(), 31..32);
//!
//!     lexer.advance();
//!
//!     assert_eq!(lexer.token, Token::End);
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate core as std;

#[cfg(feature = "export_derive")]
extern crate logos_derive;

#[cfg(feature = "export_derive")]
pub use logos_derive::Logos;

#[cfg(feature = "nul_term_source")]
extern crate toolshed;

mod lexer;
mod source;

#[doc(hidden)]
pub mod internal;

pub use lexer::{Lexer, Lexicon, Extras};
pub use source::Source;

/// Trait implemented for an enum representing all tokens. You should never have
/// to implement it manually, use the `#[derive(Logos)]` attribute on your enum.
pub trait Logos: Sized {
    /// Associated type `Extras` for the particular lexer. Those can handle things that
    /// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
    /// in JavaScript.
    type Extras: self::Extras;

    /// `SIZE` is simply a number of possible variants of the `Logos` enum. The
    /// `derive` macro will make sure that all variants don't hold values larger
    /// or equal to `SIZE`.
    ///
    /// This can be extremely useful for creating `Logos` Lookup Tables.
    const SIZE: usize;

    /// Helper `const` of the variant marked as `#[error]`.
    const ERROR: Self;

    /// Returns a lookup table for the `Lexer`
    fn lexicon<'lexicon, 'source, Source>() -> &'lexicon Lexicon<Lexer<Self, Source>>
    where
        Source: self::Source<'source>;

    /// Create a new instance of a `Lexer` that will produce tokens implementing
    /// this `Logos`.
    fn lexer<'source, Source>(source: Source) -> Lexer<Self, Source>
    where
        Source: self::Source<'source>,
    {
        Lexer::new(source)
    }
}
