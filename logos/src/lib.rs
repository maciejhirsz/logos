//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! **Logos** works by:
//!
//! + Resolving all logical branching of token definitions into a state machine.
//! + Optimizing complex patterns into [Lookup Tables](https://en.wikipedia.org/wiki/Lookup_table).
//! + Avoiding backtracking, unwinding loops, and batching reads to minimize bounds checking.
//!
//! In practice it means that for most grammars the lexing performance is virtually unaffected by the number
//! of tokens defined in the grammar. Or, in other words, **it is really fast**.
//!
//! ## Example
//!
//! ```rust
//! use logos::Logos;
//!
//! #[derive(Logos, Debug, PartialEq)]
//! enum Token {
//!     // Logos requires one token variant to handle errors,
//!     // it can be named anything you wish.
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
//!     let mut lex = Token::lexer("Create ridiculously fast Lexers.");
//!
//!     assert_eq!(lex.next(), Some(Token::Text));
//!     assert_eq!(lex.span(), 0..6);
//!     assert_eq!(lex.slice(), "Create");
//!
//!     assert_eq!(lex.next(), Some(Token::Text));
//!     assert_eq!(lex.span(), 7..19);
//!     assert_eq!(lex.slice(), "ridiculously");
//!
//!     assert_eq!(lex.next(), Some(Token::Fast));
//!     assert_eq!(lex.span(), 20..24);
//!     assert_eq!(lex.slice(), "fast");
//!
//!     assert_eq!(lex.next(), Some(Token::Text));
//!     assert_eq!(lex.span(), 25..31);
//!     assert_eq!(lex.slice(), "Lexers");
//!
//!     assert_eq!(lex.next(), Some(Token::Period));
//!     assert_eq!(lex.span(), 31..32);
//!     assert_eq!(lex.slice(), ".");
//!
//!     assert_eq!(lex.next(), None);
//! }
//! ```
//!
//! ### Callbacks
//!
//! On top of using the enum variants, **Logos** can also call arbitrary functions whenever a pattern is matched,
//! which can be used to put data into a variant:
//!
//! ```rust
//! use logos::{Logos, Lexer, Extras};
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
//! enum Token {
//!     #[error]
//!     Error,
//!
//!     // Callbacks can use closure syntax, or refer
//!     // to a function defined elsewhere.
//!     //
//!     // Each pattern can have it's own callback.
//!     #[regex("[0-9]+", |lex| lex.slice().parse())]
//!     #[regex("[0-9]+k", kilo)]
//!     #[regex("[0-9]+m", mega)]
//!     Number(u64),
//! }
//!
//! fn main() {
//!     let mut lex = Token::lexer("5 42k 75m");
//!
//!     assert_eq!(lex.next(), Some(Token::Number(5)));
//!     assert_eq!(lex.slice(), "5");
//!
//!     assert_eq!(lex.next(), Some(Token::Number(42_000)));
//!     assert_eq!(lex.slice(), "42k");
//!
//!     assert_eq!(lex.next(), Some(Token::Number(75_000_000)));
//!     assert_eq!(lex.slice(), "75m");
//!
//!     assert_eq!(lex.next(), None);
//! }
//! ```
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

#[cfg(not(feature = "std"))]
extern crate core as std;

#[cfg(feature = "export_derive")]
pub use logos_derive::Logos;

mod lexer;
pub mod source;

#[doc(hidden)]
pub mod internal;

pub use crate::lexer::{Extras, Lexer, Span, SpannedIter};
pub use crate::source::Source;

/// Trait implemented for an enum representing all tokens. You should never have
/// to implement it manually, use the `#[derive(Logos)]` attribute on your enum.
pub trait Logos<'source>: Sized {
    /// Associated type `Extras` for the particular lexer. Those can handle things that
    /// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
    /// in JavaScript.
    type Extras: Extras;

    /// Source type this token can be lexed from. This will default to `str`,
    /// unless one of the defined patterns explicitly uses non-unicode byte values
    /// or byte slices, in which case that implementation will use `[u8]`.
    type Source: Source + ?Sized + 'source;

    /// `SIZE` is simply a number of possible variants of the `Logos` enum. The
    /// `derive` macro will make sure that all variants don't hold values larger
    /// or equal to `SIZE`.
    ///
    /// This can be extremely useful for creating `Logos` Lookup Tables.
    const SIZE: usize;

    /// Helper `const` of the variant marked as `#[error]`.
    const ERROR: Self;

    /// The heart of Logos. Called by the `Lexer`. The implementation for this function
    /// is generated by the `logos-derive` crate.
    fn lex(lexer: &mut Lexer<'source, Self>);

    /// Create a new instance of a `Lexer` that will produce tokens implementing
    /// this `Logos`.
    fn lexer(source: &'source Self::Source) -> Lexer<'source, Self> {
        Lexer::new(source)
    }
}

/// Macro for creating lookup tables where index matches the token variant
/// as `usize`.
///
/// This can be especially useful for creating Jump Tables using the static `fn()`
/// function pointers, enabling an O(1) branching at the cost of introducing some
/// indirection.
///
/// ```rust
/// use logos::{Logos, lookup};
///
/// #[derive(Logos, Clone, Copy, PartialEq, Debug)]
/// enum Token {
///     #[error]
///     Error,
///
///     #[token = "Immanetize"]
///     Immanetize,
///
///     #[token = "the"]
///     The,
///
///     #[token = "Eschaton"]
///     Eschaton,
/// }
///
/// static LUT: [fn(u32) -> u32; Token::SIZE] = lookup! {
///     // Rust is smart enough to convert closure syntax to `fn()`
///     // pointers here, as long as we don't capture any values.
///     Token::Eschaton => |n| n + 40,
///     Token::Immanetize => |n| n + 8999,
///     _ => (|_| 0) as fn(u32) -> u32, // Might have to hint the type
/// };
///
/// fn main() {
///     let mut lexer = Token::lexer("Immanetize the Eschaton");
///
///     let mut token = lexer.next().unwrap();
///     assert_eq!(token, Token::Immanetize);
///     assert_eq!(LUT[token as usize](2), 9001); // 2 + 8999
///
///     token = lexer.next().unwrap();
///     assert_eq!(token, Token::The);
///     assert_eq!(LUT[token as usize](2), 0); // always 0
///
///     token = lexer.next().unwrap();
///     assert_eq!(token, Token::Eschaton);
///     assert_eq!(LUT[token as usize](2), 42); // 2 + 40
/// }
/// ```
#[macro_export]
macro_rules! lookup {
    ( $enum:ident::$variant:ident => $value:expr, $( $e:ident::$var:ident => $val:expr ,)* _ => $def:expr $(,)? ) => ({
        let mut table = [$def; $enum::SIZE];

        table[$enum::$variant as usize] = $value;
        $(
            table[$e::$var as usize] = $val;
        )*

        table
    })
}
