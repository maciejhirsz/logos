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
//! See the documentation of the [`Logos`](derive@Logos) derive macro for a
//! reference of all the attributes it accepts, and the
//! [Logos handbook](https://maciejhirsz.github.io/logos/) for additional
//! documentation and usage examples.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![doc(html_logo_url = "https://maciej.codes/kosz/logos.png")]
#![cfg_attr(feature = "forbid_unsafe", forbid(unsafe_code))]

extern crate core;

use core::fmt::Debug;

/// Derive macro generating an implementation of the [`Logos`](trait@Logos) trait
/// for an `enum` of tokens.
///
/// Each variant of the `enum` describes one kind of token. The patterns that
/// produce it are declared with [`#[token]` and `#[regex]`](#token-and-regex)
/// attributes on the variant, and the lexer as a whole can be configured with
/// [`#[logos(...)]`](#logos) attributes on the `enum` itself. The derive
/// macro compiles all patterns into a single state machine at compile time.
///
/// This page is a reference of everything the derive macro accepts. The
/// [Logos handbook](https://maciejhirsz.github.io/logos/) covers the same
/// topics in a tutorial style, with additional longer examples.
///
/// # Example
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
/// enum Token {
///     // Tokens can be literal strings, of any length.
///     #[token("fast")]
///     Fast,
///
///     #[token(".")]
///     Period,
///
///     // Or regular expressions.
///     #[regex("[a-zA-Z]+")]
///     Text,
/// }
///
/// let mut lex = Token::lexer("Create ridiculously fast Lexers.");
///
/// assert_eq!(lex.next(), Some(Ok(Token::Text)));
/// assert_eq!(lex.span(), 0..6);
/// assert_eq!(lex.slice(), "Create");
///
/// assert_eq!(lex.next(), Some(Ok(Token::Text)));
/// assert_eq!(lex.slice(), "ridiculously");
///
/// assert_eq!(lex.next(), Some(Ok(Token::Fast)));
/// assert_eq!(lex.span(), 20..24);
///
/// assert_eq!(lex.next(), Some(Ok(Token::Text)));
/// assert_eq!(lex.slice(), "Lexers");
///
/// assert_eq!(lex.next(), Some(Ok(Token::Period)));
/// assert_eq!(lex.next(), None);
/// ```
///
/// [`Logos::lexer`] returns a [`Lexer`], which is an [`Iterator`] over
/// `Result<Token, Token::Error>`: input that does not match any pattern
/// produces an `Err` (see [custom error type](#custom-error-type)).
/// [`Lexer::span`] and [`Lexer::slice`] give the position and the text of the
/// token that was just returned.
///
/// # `#[token]` and `#[regex]`
///
/// ```text
/// #[token(literal [, callback] [, priority = <integer>] [, ignore(case)])]
/// #[regex(literal [, callback] [, priority = <integer>] [, ignore(case)])]
/// ```
///
/// + `#[token]` matches the literal exactly; `#[regex]` interprets it as a
///   regular expression (the syntax of the [`regex`](https://docs.rs/regex)
///   crate, without backtracking features such as look-around or
///   back-references).
/// + `literal` can be a `&str` or a `&[u8]` (byte string) literal.
/// + `callback` is a closure or a path to a function, see [callbacks](#callbacks).
/// + `priority` overrides the computed priority, see
///   [token disambiguation](#token-disambiguation).
/// + `ignore(case)` makes the pattern match case-insensitively.
///
/// Only `literal` is required. Any number of `#[token]` and `#[regex]`
/// attributes can be stacked on the same variant.
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(skip " +")]
/// enum Token {
///     #[token("true", ignore(case))]
///     #[token("yes")]
///     True,
///
///     #[regex("-?[0-9]+")]
///     Integer,
/// }
///
/// let tokens: Vec<_> = Token::lexer("TRUE yes -42").collect();
/// assert_eq!(tokens, [Ok(Token::True), Ok(Token::True), Ok(Token::Integer)]);
/// ```
///
/// # Token disambiguation
///
/// When two or more patterns can match the same input, **Logos** picks the
/// one with the highest priority. The rule of thumb is:
///
/// + Longer beats shorter.
/// + Specific beats generic.
///
/// The priority of a pattern is computed from its shortest possible match:
/// every consecutive, non-repeating single byte adds 2, while every range or
/// class adds 1. Loops and optional parts are ignored, and alternations count
/// their shortest alternative. For example:
///
/// + `[a-zA-Z]+` has a priority of 2 (the lowest possible);
/// + `foobar` has a priority of 12;
/// + `(foo|hello)(bar)?` has a priority of 6, `foo` being its shortest match.
///
/// If two different patterns can match the same input and have the same
/// priority, the derive macro reports a compile error. Set the priority of one
/// of them explicitly to resolve it:
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// enum Token {
///     // Both patterns have a priority of 2 and both match "a".
///     #[regex("[a-z]", priority = 3)]
///     Lower,
///
///     #[regex("[a-zA-Z]")]
///     Letter,
/// }
///
/// let tokens: Vec<_> = Token::lexer("aB").collect();
/// assert_eq!(tokens, [Ok(Token::Lower), Ok(Token::Letter)]);
/// ```
///
/// # Callbacks
///
/// A callback runs whenever its pattern matches. It receives a
/// `&mut Lexer<Token>` and can be used to put data into a variant, to validate
/// the match, to skip it, or to consume more input with [`Lexer::bump`] (see
/// also [`Lexer::remainder`]).
///
/// ```
/// use logos::{Lexer, Logos};
///
/// // Note: callbacks can return `Option` or `Result`
/// fn kilo(lex: &mut Lexer<Token>) -> Option<u64> {
///     let slice = lex.slice();
///     let n: u64 = slice[..slice.len() - 1].parse().ok()?; // skip 'k'
///     Some(n * 1_000)
/// }
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(skip r"[ \t\n\f]+")]
/// enum Token {
///     // Callbacks can use closure syntax, or refer
///     // to a function defined elsewhere.
///     //
///     // Each pattern can have its own callback.
///     #[regex("[0-9]+", |lex| lex.slice().parse().ok())]
///     #[regex("[0-9]+k", kilo)]
///     Number(u64),
/// }
///
/// let tokens: Vec<_> = Token::lexer("5 42k").collect();
/// assert_eq!(tokens, [Ok(Token::Number(5)), Ok(Token::Number(42_000))]);
/// ```
///
/// Callbacks can have the following return types (`Unit` stands for a variant
/// without fields, `Value(T)` for a variant with a single field):
///
/// | Return type            | Produces                                                                            |
/// | ---------------------- | ----------------------------------------------------------------------------------- |
/// | `()`                   | `Ok(Token::Unit)`                                                                   |
/// | `bool`                 | `Ok(Token::Unit)` **or** `Err(<Token as Logos>::Error::default())`                  |
/// | `Result<(), E>`        | `Ok(Token::Unit)` **or** `Err(<Token as Logos>::Error::from(err))`                  |
/// | `T`                    | `Ok(Token::Value(T))`                                                               |
/// | `Option<T>`            | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::default())`              |
/// | `Result<T, E>`         | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::from(err))`              |
/// | [`Skip`]               | _skips matched input_                                                               |
/// | `Result<Skip, E>`      | _skips matched input_ **or** `Err(<Token as Logos>::Error::from(err))`              |
/// | [`Filter<T>`]          | `Ok(Token::Value(T))` **or** _skips matched input_                                  |
/// | [`FilterResult<T, E>`] | `Ok(Token::Value(T))` **or** `Err(<Token as Logos>::Error::from(err))` **or** _skips matched input_ |
///
/// # `#[logos]`
///
/// All `#[logos(...)]` attributes are optional and go on the `enum`:
///
/// ```text
/// #[logos(skip "regex literal")]
/// #[logos(skip("regex literal" [, callback] [, priority = <integer>]))]
/// #[logos(extras = ExtrasType)]
/// #[logos(error = ErrorType)]
/// #[logos(error(ErrorType, callback))]
/// #[logos(utf8 = false)]
/// #[logos(lifetime = 'a)]
/// #[logos(lifetime = none)]
/// #[logos(subpattern name = "regex literal")]
/// #[logos(type T = SomeType)]
/// #[logos(crate = path::to::logos)]
/// #[logos(export_dir = "path/to/dir")]
/// ```
///
/// ## Skipping input
///
/// `#[logos(skip "regex")]` ignores input matching the regex between tokens,
/// typically whitespace or comments. It accepts the same optional `callback`
/// and `priority` arguments as `#[regex]`; the callback should return [`Skip`]
/// or `()`.
///
/// ## Extras
///
/// Each [`Lexer`] has a public `extras` field that callbacks can read and
/// mutate, to carry state between tokens. Its type is `()` by default and can
/// be set with `#[logos(extras = ExtrasType)]`. The [`Logos::lexer`]
/// constructor requires `ExtrasType: Default`; use
/// [`Logos::lexer_with_extras`] to provide an initial value.
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(extras = usize)] // line number
/// #[logos(skip(r"\n", |lex| lex.extras += 1))]
/// #[logos(skip " +")]
/// enum Token {
///     #[regex("[a-z]+", |lex| lex.extras)]
///     Word(usize),
/// }
///
/// let tokens: Vec<_> = Token::lexer("foo\nbar baz\n\nqux").collect();
/// assert_eq!(
///     tokens,
///     [Ok(Token::Word(0)), Ok(Token::Word(1)), Ok(Token::Word(1)), Ok(Token::Word(3))],
/// );
/// ```
///
/// ## Custom error type
///
/// By default the error type is `()`. It can be changed with
/// `#[logos(error = ErrorType)]`, where `ErrorType` must implement `Default`,
/// `Clone`, `PartialEq` and `Debug`, as well as `From<E>` for the error type
/// `E` of every callback returning a `Result`. Input that matches no pattern
/// produces `Err(ErrorType::default())`.
///
/// To build the error from the lexer instead (e.g. to record the span or the
/// offending text), use `#[logos(error(ErrorType, callback))]`, where the
/// callback takes `&mut Lexer<Token>` and returns `ErrorType`:
///
/// ```
/// use logos::Logos;
/// use std::num::ParseIntError;
///
/// #[derive(Default, Debug, Clone, PartialEq)]
/// enum LexingError {
///     InvalidInteger(ParseIntError),
///     UnexpectedCharacter(char),
///     #[default]
///     Other,
/// }
///
/// impl From<ParseIntError> for LexingError {
///     fn from(err: ParseIntError) -> Self {
///         LexingError::InvalidInteger(err)
///     }
/// }
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(error(LexingError, |lex| {
///     LexingError::UnexpectedCharacter(lex.slice().chars().next().unwrap())
/// }))]
/// #[logos(skip " +")]
/// enum Token {
///     #[regex("[0-9]+", |lex| lex.slice().parse())]
///     Integer(u8),
/// }
///
/// let mut lex = Token::lexer("42 256 !");
/// assert_eq!(lex.next(), Some(Ok(Token::Integer(42))));
/// assert!(matches!(lex.next(), Some(Err(LexingError::InvalidInteger(_)))));
/// assert_eq!(lex.next(), Some(Err(LexingError::UnexpectedCharacter('!'))));
/// assert_eq!(lex.next(), None);
/// ```
///
/// ## Byte input
///
/// By default the lexer accepts `&str` input, and patterns that can match
/// invalid UTF-8 cause a compile error. With `#[logos(utf8 = false)]` the
/// lexer accepts `&[u8]` instead:
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(utf8 = false)]
/// enum Token {
///     #[token(b"\xCA\xFE")]
///     Magic,
///
///     #[regex(b"[\x00-\x7F]+")]
///     Ascii,
/// }
///
/// let tokens: Vec<_> = Token::lexer(b"\xCA\xFEhi").collect();
/// assert_eq!(tokens, [Ok(Token::Magic), Ok(Token::Ascii)]);
/// ```
///
/// ## Source lifetime
///
/// Variants can borrow from the input. When the token type has a single
/// lifetime parameter, it is used as the lifetime of the source:
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(skip " +")]
/// enum Token<'s> {
///     #[regex("[a-z]+")]
///     Word(&'s str),
/// }
///
/// let tokens: Vec<_> = Token::lexer("hello world").collect();
/// assert_eq!(tokens, [Ok(Token::Word("hello")), Ok(Token::Word("world"))]);
/// ```
///
/// If the token type has several lifetimes, the source lifetime must be chosen
/// with `#[logos(lifetime = 'a)]`. `#[logos(lifetime = none)]` makes the
/// macro introduce a new source lifetime instead of using one of the token's.
///
/// ## Subpatterns
///
/// `#[logos(subpattern name = "regex")]` defines a named regex that can be
/// reused in other patterns (and subpatterns) with `(?&name)`:
///
/// ```
/// use logos::Logos;
///
/// #[derive(Logos, Debug, PartialEq)]
/// #[logos(skip r"\s+")]
/// #[logos(subpattern alpha = r"[a-zA-Z]")]
/// #[logos(subpattern digit = r"[0-9]")]
/// #[logos(subpattern alphanum = r"(?&alpha)|(?&digit)")]
/// enum Token {
///     #[regex("(?&alpha)+")]
///     Word,
///     #[regex("(?&digit)+")]
///     Number,
///     #[regex("(?&alphanum){2}")]
///     TwoAlphanum,
///     #[regex("(?&alphanum){3}")]
///     ThreeAlphanum,
/// }
///
/// let mut lex = Token::lexer("Word 1234 ab3 12");
/// assert_eq!(lex.next(), Some(Ok(Token::Word)));
/// assert_eq!(lex.next(), Some(Ok(Token::Number)));
/// assert_eq!(lex.next(), Some(Ok(Token::ThreeAlphanum)));
/// assert_eq!(lex.next(), Some(Ok(Token::TwoAlphanum)));
/// assert_eq!(lex.next(), None);
/// ```
///
/// ## Other options
///
/// + `#[logos(type T = SomeType)]` sets a concrete type for a generic type
///   parameter `T` of the token `enum`.
/// + `#[logos(crate = path::to::logos)]` makes the generated code refer to the
///   `logos` crate through a different path (e.g. when it is re-exported by
///   another crate).
/// + `#[logos(export_dir = "path/to/dir")]` writes the generated state machine
///   as Graphviz (`.dot`) and Mermaid (`.mmd`) graphs at compile time, which
///   helps with debugging. The path may also end in `.dot` or `.mmd` to export
///   a single format.
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
    fn lex(lexer: &mut Lexer<'source, Self>) -> Option<Result<Self, Self::Error>>;

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
#[doc = include_str!("../README.md")]
struct ReadMe;
