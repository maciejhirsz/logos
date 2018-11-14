use std::ops::Range;

use super::{Logos, Source};
use super::internal::LexerInternal;

/// A Lookup Table used internally. It maps indices for every valid
/// byte to a function that takes a mutable reference to the `Lexer`,
/// reads the input and sets the correct token variant for it.
pub type Lexicon<Lexer> = [Option<fn(&mut Lexer)>; 256];

/// `Lexer` is the main struct of the crate that allows you to read through a
/// `Source` and produce tokens for enums implementing the `Logos` trait.
pub struct Lexer<Token: Logos, Source> {
    source: Source,
    token_start: usize,
    token_end: usize,
    lexicon: Lexicon<Lexer<Token, Source>>,

    /// Current token. Call the `advance` method to get a new token.
    pub token: Token,

    /// Extras associated with the `Token`.
    pub extras: Token::Extras,

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

impl<Token: Logos, Src: Source> Lexer<Token, Src> {
    /// Create a new `Lexer`.
    ///
    /// Due to type inference, it might be more ergonomic to construct
    /// it by calling `Token::lexer(source)`, where `Token` implements `Logos`.
    pub fn new(source: Src) -> Self {
        let mut lex = Lexer {
            source,
            token_start: 0,
            token_end: 0,
            lexicon: Token::lexicon(),
            token: Token::ERROR,
            extras: Default::default(),
        };

        lex.advance();

        lex
    }

    /// Advance the `Lexer` and attempt to produce the next `Token`.
    pub fn advance(&mut self) {
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

    /// Get the range for the current token in `Source`.
    pub fn range(&self) -> Range<usize> {
        self.token_start .. self.token_end
    }

    /// Get a string slice of the current token.
    pub fn slice(&self) -> Src::Slice {
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

#[doc(hidden)]
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
impl<Token: Logos, S: Source> LexerInternal<Token> for Lexer<Token, S> {
    /// Read a byte at current position of the `Lexer`. If end
    /// of the `Source` has been reached, this will return `0`.
    ///
    /// # WARNING!
    ///
    /// This should never be called as public API, and is instead
    /// meant to be called by the implementor of the `Logos` trait.
    fn read(&self) -> u8 {
        unsafe { self.source.read(self.token_end) }
    }

    /// Convenience method that bumps the position `Lexer` is
    /// reading from and then reads the following byte.
    ///
    /// # WARNING!
    ///
    /// This should never be called as public API, and is instead
    /// meant to be called by the implementor of the `Logos` trait.
    ///
    /// **If the end position has been reached, further bumps
    /// can lead to undefined behavior!**
    ///
    /// **This method will panic in debug mode if that happens!**
    fn next(&mut self) -> u8 {
        self.bump();
        self.read()
    }

    /// Bump the position `Lexer` is reading from by `1`.
    ///
    /// # WARNING!
    ///
    /// This should never be called as public API, and is instead
    /// meant to be called by the implementor of the `Logos` trait.
    ///
    /// **If the end position has been reached, further bumps
    /// can lead to undefined behavior!**
    ///
    /// **This method will panic in debug mode if that happens!**
    fn bump(&mut self) {
        debug_assert!(self.token_end + 1 <= self.source.len(), "Bumping out of bounds!");

        self.token_end += 1;
    }
}
