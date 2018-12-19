use std::ops::Range;

use crate::source::{Source, ByteArray};
use super::{Logos};
use super::internal::LexerInternal;

/// A Lookup Table used internally. It maps indices for every valid
/// byte to a function that takes a mutable reference to the `Lexer`,
/// reads the input and sets the correct token variant for it.
pub type Lexicon<Lexer> = [Option<fn(&mut Lexer)>; 256];

/// `Lexer` is the main struct of the crate that allows you to read through a
/// `Source` and produce tokens for enums implementing the `Logos` trait.
#[derive(Clone)]
pub struct Lexer<Token: Logos, Source> {
    /// Source from which the Lexer is reading tokens.
    pub source: Source,

    /// Current token. Call the `advance` method to get a new token.
    pub token: Token,

    /// Extras associated with the `Token`.
    pub extras: Token::Extras,

    token_start: usize,
    token_end: usize,
}

macro_rules! unroll {
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

impl<'source, Token, Source> Lexer<Token, Source>
where
    Token: self::Logos,
    Source: self::Source<'source>,
{
    /// Create a new `Lexer`.
    ///
    /// Due to type inference, it might be more ergonomic to construct
    /// it by calling `Token::lexer(source)`, where `Token` implements `Logos`.
    pub fn new(source: Source) -> Self {
        let mut lex = Lexer {
            source,
            token: Token::ERROR,
            extras: Default::default(),
            token_start: 0,
            token_end: 0,
        };

        lex.advance();

        lex
    }

    /// Advance the `Lexer` and attempt to produce the next `Token`.
    pub fn advance(&mut self) {
        let mut ch;

        self.extras.on_advance();

        unroll! {
            ch = self.read();

            if let Some(handler) = Token::lexicon()[ch as usize] {
                self.token_start = self.token_end;

                return handler(self);
            }

            self.extras.on_whitespace(ch);
            self.bump(1);
        }
    }

    /// Get the range for the current token in `Source`.
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.token_start .. self.token_end
    }

    /// Get a string slice of the current token.
    #[inline]
    pub fn slice(&self) -> Source::Slice {
        unsafe { self.source.slice_unchecked(self.range()) }
    }
}

impl<'source, Token, Source> Lexer<Token, Source>
where
    Token: self::Logos,
    Source: self::Source<'source>,
{
    /// Turn this lexer into a lexer for a new token type.
    ///
    /// The new lexer continues to point at the same span as the current lexer,
    /// and the current token becomes the error token of the new token type.
    /// If you want to start reading from the new lexer immediately,
    /// consider using `Lexer::advance_as` instead.
    pub fn morph<Token2: Logos>(self) -> Lexer<Token2, Source>
    where
        Token::Extras: Into<Token2::Extras>,
    {
        Lexer {
            source: self.source,
            token: Token2::ERROR,
            extras: self.extras.into(),
            token_start: self.token_start,
            token_end: self.token_end,
        }
    }

    /// Advance the `Lexer` and attempt to produce the next `Token` of a new token type.
    ///
    /// This function takes self by value as a lint. If you're working with a `&mut Lexer`,
    /// clone the old lexer to call this method, then don't forget to update the old lexer!
    pub fn advance_as<Token2: Logos>(self) -> Lexer<Token2, Source>
    where
        Token::Extras: Into<Token2::Extras>,
    {
        let mut lex = self.morph();
        lex.advance();
        lex
    }
}

/// Helper trait that can be injected into the `Lexer` to handle things that
/// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
/// in JavaScript.
pub trait Extras: Sized + Default {
    /// Method called by the `Lexer` when a new token is about to be produced.
    #[inline]
    fn on_advance(&mut self) {}

    /// Method called by the `Lexer` when a white space byte has been encountered.
    #[inline]
    fn on_whitespace(&mut self, _byte: u8) {}
}

/// Default `Extras` with no logic
impl Extras for () { }

#[doc(hidden)]
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
impl<'source, Token, Source> LexerInternal<'source> for Lexer<Token, Source>
where
    Token: self::Logos,
    Source: self::Source<'source>,
{
    /// Read a byte at current position of the `Lexer`. If end
    /// of the `Source` has been reached, this will return `0`.
    ///
    /// # WARNING!
    ///
    /// This should never be called as public API, and is instead
    /// meant to be called by the implementor of the `Logos` trait.
    #[inline]
    fn read(&self) -> u8 {
        unsafe { self.source.read(self.token_end) }
    }

    #[inline]
    fn read_bytes<Array>(&self) -> Option<&'source Array>
    where
        Array: ByteArray<'source>
    {
        self.source.read_bytes(self.token_end)
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
    #[inline]
    fn next(&mut self) -> u8 {
        self.bump(1);
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
    #[inline]
    fn bump(&mut self, size: usize) {
        debug_assert!(self.token_end + size <= self.source.len(), "Bumping out of bounds!");

        self.token_end += size;
    }
}
