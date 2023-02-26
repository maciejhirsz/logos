//! <img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.svg?sanitize=true" alt="Logos logo" width="250" align="right">
//!
//! # Logos
//!
//! _Create ridiculously fast Lexers._
//!
//! **Logos** has two goals:
//!
//! + To make it easy to create a Lexer, so you can focus on more complex problems.
//! + To make the generated Lexer faster than anything you'd write by hand.
//!
//! To achieve those, **Logos**:
//!
//! + Combines all token definitions into a single [deterministic state machine](https://en.wikipedia.org/wiki/Deterministic_finite_automaton).
//! + Optimizes branches into [lookup tables](https://en.wikipedia.org/wiki/Lookup_table) or [jump tables](https://en.wikipedia.org/wiki/Branch_table).
//! + Prevents [backtracking](https://en.wikipedia.org/wiki/ReDoS) inside token definitions.
//! + [Unwinds loops](https://en.wikipedia.org/wiki/Loop_unrolling), and batches reads to minimize bounds checking.
//! + Does all of that heavy lifting at compile time.
//!
//! ## Example
//!
//! ```rust
//! use logos::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! #[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
//! enum Token {
//!     // Tokens can be literal strings, of any length.
//!     #[token("fast")]
//!     Fast,
//!
//!     #[token(".")]
//!     Period,
//!
//!     // Or regular expressions.
//!     #[regex("[a-zA-Z]+")]
//!     Text,
//! }
//!
//! fn main() {
//!     let mut lex = Token::lexer("Create ridiculously fast Lexers.");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Text)));
//!     assert_eq!(lex.span(), 0..6);
//!     assert_eq!(lex.slice(), "Create");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Text)));
//!     assert_eq!(lex.span(), 7..19);
//!     assert_eq!(lex.slice(), "ridiculously");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Fast)));
//!     assert_eq!(lex.span(), 20..24);
//!     assert_eq!(lex.slice(), "fast");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Text)));
//!     assert_eq!(lex.slice(), "Lexers");
//!     assert_eq!(lex.span(), 25..31);
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Period)));
//!     assert_eq!(lex.span(), 31..32);
//!     assert_eq!(lex.slice(), ".");
//!
//!     assert_eq!(lex.next(), None);
//! }
//! ```
//!
//! ### Callbacks
//!
//! **Logos** can also call arbitrary functions whenever a pattern is matched,
//! which can be used to put data into a variant:
//!
//! ```rust
//! use logos::{Logos, Lexer};
//!
//! // Note: callbacks can return `Option` or `Result`
//! fn kilo(lex: &mut Lexer<Token>) -> Option<u64> {
//!     let slice = lex.slice();
//!     let n: u64 = slice[..slice.len() - 1].parse().ok()?; // skip 'k'
//!     Some(n * 1_000)
//! }
//!
//! fn mega(lex: &mut Lexer<Token>) -> Option<u64> {
//!     let slice = lex.slice();
//!     let n: u64 = slice[..slice.len() - 1].parse().ok()?; // skip 'm'
//!     Some(n * 1_000_000)
//! }
//!
//! #[derive(Logos, Debug, PartialEq)]
//! #[logos(skip r"[ \t\n\f]+")]
//! enum Token {
//!     // Callbacks can use closure syntax, or refer
//!     // to a function defined elsewhere.
//!     //
//!     // Each pattern can have it's own callback.
//!     #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
//!     #[regex("[0-9]+k", kilo)]
//!     #[regex("[0-9]+m", mega)]
//!     Number(u64),
//! }
//!
//! fn main() {
//!     let mut lex = Token::lexer("5 42k 75m");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Number(5))));
//!     assert_eq!(lex.slice(), "5");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Number(42_000))));
//!     assert_eq!(lex.slice(), "42k");
//!
//!     assert_eq!(lex.next(), Some(Ok(Token::Number(75_000_000))));
//!     assert_eq!(lex.slice(), "75m");
//!
//!     assert_eq!(lex.next(), None);
//! }
//! ```
//!
//! Logos can handle callbacks with following return types:
//!
//! | Return type                                      | Produces                                                                                            |
//! |--------------------------------------------------|-----------------------------------------------------------------------------------------------------|
//! | `()`                                             | `Ok(Token::Unit)`                                                                                   |
//! | `bool`                                           | `Ok(Token::Unit)` **or** `Err(<Token as Logos>::Error::default())`                                  |
//! | `Result<(), E>`                                  | `Ok(Token::Unit)` **or** `Err(<Token as Logos>::Error::from(err))`                                  |
//! | `T`                                              | `Ok(Token::Value(T))`                                                                               |
//! | `Option<T>`                                      | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::default())`                              |
//! | `Result<T, E>`                                   | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::from(err))`                              |
//! | [`Skip`](./struct.Skip.html)                     | _skips matched input_                                                                               |
//! | [`Filter<T>`](./enum.Filter.html)                | `Ok(Token::Value(T))` **or** _skips matched input_                                                  |
//! | [`FilterResult<T, E>`](./enum.FilterResult.html) | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::from(err))` **or** _skips matched input_ |
//!
//! Callbacks can be also used to do perform more specialized lexing in place
//! where regular expressions are too limiting. For specifics look at
//! [`Lexer::remainder`](./struct.Lexer.html#method.remainder) and
//! [`Lexer::bump`](./struct.Lexer.html#method.bump).
//!
//! ## Errors
//!
//! By default, **Logos** uses `()` as the error type, which means that it
//! doesn't store any information about the error.
//! This can be changed by using `#[logos(error = T)]` attribute on the enum.
//! The type `T` can be any type that implements `Clone`, `PartialEq`,
//! `Default` and `From<E>` for each callback's error type.
//!
//! ## Token disambiguation
//!
//! Rule of thumb is:
//!
//! + Longer beats shorter.
//! + Specific beats generic.
//!
//! If any two definitions could match the same input, like `fast` and `[a-zA-Z]+`
//! in the example above, it's the longer and more specific definition of `Token::Fast`
//! that will be the result.
//!
//! This is done by comparing numeric priority attached to each definition. Every consecutive,
//! non-repeating single byte adds 2 to the priority, while every range or regex class adds 1.
//! Loops or optional blocks are ignored, while alternations count the shortest alternative:
//!
//! + `[a-zA-Z]+` has a priority of 1 (lowest possible), because at minimum it can match a single byte to a class.
//! + `foobar` has a priority of 12.
//! + `(foo|hello)(bar)?` has a priority of 6, `foo` being it's shortest possible match.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![doc(html_logo_url = "https://maciej.codes/kosz/logos.png")]

#[cfg(not(feature = "std"))]
extern crate core as std;

#[cfg(feature = "export_derive")]
pub use logos_derive::Logos;
use std::fmt::Debug;

mod lexer;
pub mod source;

#[doc(hidden)]
pub mod internal;

pub use crate::lexer::{Lexer, Span, SpannedIter};
pub use crate::source::Source;

/// Trait implemented for an enum representing all tokens. You should never have
/// to implement it manually, use the `#[derive(Logos)]` attribute on your enum.
pub trait Logos<'source>: Sized {
    /// Associated type `Extras` for the particular lexer. This can be set using
    /// `#[logos(extras = MyExtras)]` and accessed inside callbacks.
    type Extras;

    /// Source type this token can be lexed from. This will default to `str`,
    /// unless one of the defined patterns explicitly uses non-unicode byte values
    /// or byte slices, in which case that implementation will use `[u8]`.
    type Source: Source + ?Sized + 'source;

    /// Error type returned by the lexer. This can be set using
    /// `#[logos(error = MyError)]`. Defaults to `()` if not set.
    type Error: Default + Clone + PartialEq + Debug + 'source;

    /// The heart of Logos. Called by the `Lexer`. The implementation for this function
    /// is generated by the `logos-derive` crate.
    fn lex(lexer: &mut Lexer<'source, Self>);

    /// Create a new instance of a `Lexer` that will produce tokens implementing
    /// this `Logos`.
    fn lexer(source: &'source Self::Source) -> Lexer<'source, Self>
    where
        Self::Extras: Default,
    {
        Lexer::new(source)
    }

    /// Create a new instance of a `Lexer` with the provided `Extras` that will
    /// produce tokens implementing this `Logos`.
    fn lexer_with_extras(
        source: &'source Self::Source,
        extras: Self::Extras,
    ) -> Lexer<'source, Self> {
        Lexer::with_extras(source, extras)
    }
}

/// Type that can be returned from a callback, informing the `Lexer`, to skip
/// current token match. See also [`logos::skip`](./fn.skip.html).
///
/// # Example
///
/// ```rust
/// use logos::{Logos, Skip};
///
/// #[derive(Logos, Debug, PartialEq)]
/// enum Token<'a> {
///     // We will treat "abc" as if it was whitespace.
///     // This is identical to using `logos::skip`.
///     #[regex(" |abc", |_| Skip)]
///     Ignored,
///
///     #[regex("[a-zA-Z]+")]
///     Text(&'a str),
/// }
///
/// let tokens: Vec<_> = Token::lexer("Hello abc world").collect();
///
/// assert_eq!(
///     tokens,
///     &[
///         Ok(Token::Text("Hello")),
///         Ok(Token::Text("world")),
///     ],
/// );
/// ```
pub struct Skip;

/// Type that can be returned from a callback, either producing a field
/// for a token, or skipping it.
///
/// # Example
///
/// ```rust
/// use logos::{Logos, Filter};
///
/// #[derive(Logos, Debug, PartialEq)]
/// enum Token {
///     #[regex(r"[ \n\f\t]+", logos::skip)]
///     Ignored,
///
///     #[regex("[0-9]+", |lex| {
///         let n: u64 = lex.slice().parse().unwrap();
///
///         // Only emit a token if `n` is an even number
///         match n % 2 {
///             0 => Filter::Emit(n),
///             _ => Filter::Skip,
///         }
///     })]
///     EvenNumber(u64)
/// }
///
/// let tokens: Vec<_> = Token::lexer("20 11 42 23 100 8002").collect();
///
/// assert_eq!(
///     tokens,
///     &[
///         Ok(Token::EvenNumber(20)),
///         // skipping 11
///         Ok(Token::EvenNumber(42)),
///         // skipping 23
///         Ok(Token::EvenNumber(100)),
///         Ok(Token::EvenNumber(8002))
///     ]
/// );
/// ```
pub enum Filter<T> {
    /// Emit a token with a given value `T`. Use `()` for unit variants without fields.
    Emit(T),
    /// Skip current match, analog to [`Skip`](./struct.Skip.html).
    Skip,
}

/// Type that can be returned from a callback, either producing a field
/// for a token, skipping it, or emitting an error.
///
/// # Example
///
/// ```rust
/// use logos::{Logos, FilterResult};
///
/// #[derive(Debug, PartialEq, Clone, Default)]
/// enum LexingError {
///     NumberParseError,
///     NumberIsTen,
///     #[default]
///     Other,
/// }
///
/// impl From<std::num::ParseIntError> for LexingError {
///     fn from(_: std::num::ParseIntError) -> Self {
///         LexingError::NumberParseError
///     }
/// }
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(error = LexingError)]
/// enum Token {
///     #[regex(r"[ \n\f\t]+", logos::skip)]
///     Ignored,
///
///     #[regex("[0-9]+", |lex| {
///         let n: u64 = lex.slice().parse().unwrap();
///
///         // Only emit a token if `n` is an even number.
///         if n % 2 == 0 {
///             // Emit an error if `n` is 10.
///             if n == 10 {
///                 FilterResult::Error(LexingError::NumberIsTen)
///             } else {
///                 FilterResult::Emit(n)
///             }
///         } else {
///             FilterResult::Skip
///         }
///     })]
///     NiceEvenNumber(u64)
/// }
///
/// let tokens: Vec<_> = Token::lexer("20 11 42 23 100 10").collect();
///
/// assert_eq!(
///     tokens,
///     &[
///         Ok(Token::NiceEvenNumber(20)),
///         // skipping 11
///         Ok(Token::NiceEvenNumber(42)),
///         // skipping 23
///         Ok(Token::NiceEvenNumber(100)),
///         // error at 10
///         Err(LexingError::NumberIsTen),
///     ]
/// );
/// ```
pub enum FilterResult<T, E> {
    /// Emit a token with a given value `T`. Use `()` for unit variants without fields.
    Emit(T),
    /// Skip current match, analog to [`Skip`](./struct.Skip.html).
    Skip,
    /// Emit a `<Token as Logos>::ERROR` token.
    Error(E),
}

/// Predefined callback that will inform the `Lexer` to skip a definition.
///
/// # Example
///
/// ```rust
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// enum Token<'a> {
///     // We will treat "abc" as if it was whitespace
///     #[regex(" |abc", logos::skip)]
///     Ignored,
///
///     #[regex("[a-zA-Z]+")]
///     Text(&'a str),
/// }
///
/// let tokens: Vec<_> = Token::lexer("Hello abc world").collect();
///
/// assert_eq!(
///     tokens,
///     &[
///         Ok(Token::Text("Hello")),
///         Ok(Token::Text("world")),
///     ],
/// );
/// ```
#[inline]
pub fn skip<'source, Token: Logos<'source>>(_: &mut Lexer<'source, Token>) -> Skip {
    Skip
}

#[cfg(doctest)]
mod test_readme {
    macro_rules! external_doc_test {
        ($x:expr) => {
            #[doc = $x]
            extern "C" {}
        };
    }

    external_doc_test!(include_str!("../../README.md"));
}
