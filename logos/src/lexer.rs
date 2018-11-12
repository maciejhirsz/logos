use std::ops::Range;

use super::{Logos, Source};

/// A lookup table used internally. It maps indices for every ASCII
/// byte to a function that takes a mutable reference to the `Lexer`.
pub type Lexicon<Lexer> = [Option<fn(&mut Lexer)>; 256];

/// `Lexer` is the main struct of the crate that allows you to read through a
/// `Source` and produce tokens implementing the `Logos` trait.
pub struct Lexer<Token: Logos, Source> {
    source: Source,
    token_start: usize,
    token_end: usize,
    pub token: Token,
    pub extras: Token::Extras,
    lexicon: Lexicon<Lexer<Token, Source>>,
}

macro_rules! unwind {
    ($( $code:tt )*) => (
        $( $code )*
        $( $code )*
        $( $code )*
        $( $code )*

        loop {
            $( $code )*
        }
    )
}

impl<Token: Logos, S: Source> Lexer<Token, S> {
    /// Create a new `Lexer`.
    ///
    /// Due to type inference, it might be more ergonomic to construct
    /// it by calling `Token::lexer(source)`, where `Token` implements `Logos`.
    pub fn new(source: S) -> Self {
        let mut lex = Lexer {
            source,
            token_start: 0,
            token_end: 0,
            token: Token::ERROR,
            extras: Default::default(),
            lexicon: Token::lexicon(),
        };

        lex.consume();

        lex
    }

    /// Get the range for the current token in `Source`.
    pub fn range(&self) -> Range<usize> {
        self.token_start .. self.token_end
    }

    /// Advance the `Lexer` and attempt to produce the next token.
    pub fn consume(&mut self) {
        let mut ch;

        self.extras.on_consume();

        unwind! {
            ch = self.read();

            if let Some(handler) = self.lexicon[ch as usize] {
                self.token_start = self.token_end;
                return handler(self);
            }

            self.extras.on_whitespace(ch);

            self.bump();
        }
    }

    /// Get a slice representing
    pub fn slice(&self) -> S::Slice {
        unsafe { self.source.slice(self.range()) }
    }
}

/// Helper trait that can be injected into the `Lexer` to handle things that
/// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
/// in JavaScript.
pub trait Extras: Sized + Default {
    /// Method called by the `Lexer` when a new token is about to be produced.
    fn on_consume(&mut self) {}

    /// Method called by the `Lexer` when a white space byte has been encountered.
    fn on_whitespace(&mut self, _byte: u8) {}
}

/// Default `Extras` with no logic
impl Extras for () { }

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<Token: Logos> {
    /// Read the byte at current position.
    fn read(&self) -> u8;

    /// Bump the position by 1 and read the following byte.
    fn next(&mut self) -> u8;

    /// Bump the position by 1.
    fn bump(&mut self);

    /// Set the token.
    fn set_token(&mut self, token: Token);
}

/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
impl<Token: Logos, S: Source> LexerInternal<Token> for Lexer<Token, S> {
    /// Read a byte at current position of the `Lexer`. If end
    /// of the `Source` has been reached, this will return `0`.
    fn read(&self) -> u8 {
        unsafe { self.source.read(self.token_end) }
    }

    /// Convenience method that bumps the position `Lexer` is
    /// reading from and then reads the following byte.
    fn next(&mut self) -> u8 {
        self.bump();
        self.read()
    }

    /// Bump the position `Lexer` is reading from by `1`.
    ///
    /// This should never be called as public API, and is instead
    /// meant to be called by the implementor of the `Logos` trait.
    ///
    /// **If the end position has been reached, further bumps
    /// can lead to undefined behavior! This method will panic
    /// in debug mode if that happens!**
    fn bump(&mut self) {
        debug_assert!(self.token_end + 1 <= self.source.len(), "Bumping out of bounds!");

        self.token_end += 1;
    }

    fn set_token(&mut self, token: Token) {
        self.token = token
    }
}
