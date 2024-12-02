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
//! See the [Logos handbook](https://maciejhirsz.github.io/logos/) for additional documentation and usage examples.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
#![doc(html_logo_url = "https://maciej.codes/kosz/logos.png")]
#![cfg_attr(feature = "forbid_unsafe", forbid(unsafe_code))]

extern crate core;

use core::fmt::Debug;
#[cfg(feature = "export_derive")]
pub use logos_derive::Logos;

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

    /// Create a new error. The default implementation uses `Error::default()`. If you want to make
    /// your own, use `#[logos(error_callback = ...)]`
    fn make_error(_lexer: &mut Lexer<'source, Self>) -> Self::Error {
        Self::Error::default()
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
///     #[regex(" |abc", |_| Skip, priority = 3)]
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
///     #[regex(" |abc", logos::skip, priority = 3)]
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

    external_doc_test!(include_str!("../README.md"));
}
