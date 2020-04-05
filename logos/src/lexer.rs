use super::internal::LexerInternal;
use super::Logos;
use crate::source::{self, Source};

/// Byte range in the source.
pub type Span = std::ops::Range<usize>;

/// `Lexer` is the main struct of the crate that allows you to read through a
/// `Source` and produce tokens for enums implementing the `Logos` trait.
pub struct Lexer<'source, Token: Logos<'source>> {
    /// Source from which the Lexer is reading tokens.
    pub source: &'source Token::Source,

    /// Extras associated with the `Token`.
    pub extras: Token::Extras,

    token: Option<Token>,
    token_start: usize,
    token_end: usize,
}

impl<'source, Token: Logos<'source>> Lexer<'source, Token> {
    /// Create a new `Lexer`.
    ///
    /// Due to type inference, it might be more ergonomic to construct
    /// it by calling [`Token::lexer`](./trait.Logos.html#method.lexer) on any `Token` with derived `Logos`.
    pub fn new(source: &'source Token::Source) -> Self {
        Lexer {
            source,
            token: None,
            extras: Default::default(),
            token_start: 0,
            token_end: 0,
        }
    }

    /// Advance the `Lexer` and attempt to produce the next `Token`.
    #[inline]
    fn advance(&mut self) {
        self.token_start = self.token_end;
        self.extras.on_advance();

        Token::lex(self);
    }

    #[inline]
    pub fn spanned(self) -> SpannedIter<'source, Token> {
        SpannedIter {
            lexer: self,
        }
    }

    #[inline]
    #[doc(hidden)]
    #[deprecated(since="0.11.0", note="please use `span` instead")]
    pub fn range(&self) -> Span {
        self.span()
    }

    /// Get the range for the current token in `Source`.
    #[inline]
    pub fn span(&self) -> Span {
        self.token_start..self.token_end
    }

    /// Get a string slice of the current token.
    #[inline]
    pub fn slice(&self) -> &'source <Token::Source as Source>::Slice {
        unsafe { self.source.slice_unchecked(self.span()) }
    }

    /// Get a slice of remaining source, starting at end of current token.
    #[inline]
    pub fn remainder(&self) -> &'source <Token::Source as Source>::Slice {
        unsafe { self.source.slice_unchecked(self.token_end..self.source.len()) }
    }

    /// Turn this lexer into a lexer for a new token type.
    ///
    /// The new lexer continues to point at the same span as the current lexer,
    /// and the current token becomes the error token of the new token type.
    /// If you want to start reading from the new lexer immediately,
    /// consider using `Lexer::advance_as` instead.
    pub fn morph<Token2>(self) -> Lexer<'source, Token2>
    where
        Token2: Logos<'source, Source = Token::Source>,
        Token::Extras: Into<Token2::Extras>,
    {
        Lexer {
            source: self.source,
            token: None,
            extras: self.extras.into(),
            token_start: self.token_start,
            token_end: self.token_end,
        }
    }

    // /// Advance the `Lexer` and attempt to produce the next `Token` of a new token type.
    // ///
    // /// This function takes self by value as a lint. If you're working with a `&mut Lexer`,
    // /// clone the old lexer to call this method, then don't forget to update the old lexer!
    // pub fn advance_as<Token2>(self) -> Lexer<'source, Token2>
    // where
    //     Token2: Logos<'source, Source = Token::Source>,
    //     Token::Extras: Into<Token2::Extras>,
    // {
    //     let mut lex = self.morph();
    //     lex.advance();
    //     lex
    // }

    /// Bumps the end of currently lexed token by `n` bytes.
    ///
    /// # Panics
    ///
    /// Panics if adding `n` to current offset would place the `Lexer` beyond the last byte,
    /// or in the middle of an UTF-8 code point (does not apply when lexing raw `&[u8]`).
    pub fn bump(&mut self, n: usize) {
        self.token_end += n;

        assert!(
            self.source.is_boundary(self.token_end),
            "Invalid Lexer bump",
        )
    }
}

impl<'source, Token> Clone for Lexer<'source, Token>
where
    Token: Logos<'source> + Clone,
    Token::Extras: Clone,
{
    fn clone(&self) -> Self {
        Lexer {
            extras: self.extras.clone(),
            token: self.token.clone(),
            ..*self
        }
    }
}

impl<'source, Token> Iterator for Lexer<'source, Token>
where
    Token: Logos<'source>,
{
    type Item = Token;

    #[inline]
    fn next(&mut self) -> Option<Token> {
        self.advance();

        self.token.take()
    }
}

/// Iterator that pairs tokens with their position in the source.
///
/// Look at [`Lexer::spanned`](./struct.Lexer.html#method.spanned) for documentation.
pub struct SpannedIter<'source, Token: Logos<'source>> {
    lexer: Lexer<'source, Token>,
}

impl<'source, Token> Iterator for SpannedIter<'source, Token>
where
    Token: Logos<'source>,
{
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|token| (
            token,
            self.lexer.span(),
        ))
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
    fn on_whitespace(&mut self) {}
}

/// Default `Extras` with no logic
impl Extras for () {}

#[doc(hidden)]
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
impl<'source, Token> LexerInternal<'source> for Lexer<'source, Token>
where
    Token: Logos<'source>,
{
    type Token = Token;

    /// Read a `Chunk` at current position of the `Lexer`. If end
    /// of the `Source` has been reached, this will return `0`.
    #[inline]
    fn read<Chunk>(&self) -> Option<Chunk>
    where
        Chunk: source::Chunk<'source>,
    {
        self.source.read(self.token_end)
    }

    /// Read a `Chunk` at a position offset by `n`.
    #[inline]
    fn read_at<Chunk>(&self, n: usize) -> Option<Chunk>
    where
        Chunk: source::Chunk<'source>,
    {
        self.source.read(self.token_end + n)
    }

    /// Test a chunk at current position with a closure.
    #[inline]
    fn test<T, F>(&self, test: F) -> bool
    where
        T: source::Chunk<'source>,
        F: FnOnce(T) -> bool,
    {
        match self.source.read::<T>(self.token_end) {
            Some(chunk) => test(chunk),
            None => false,
        }
    }

    /// Test a chunk at current position offset by `n` with a closure.
    #[inline]
    fn test_at<T, F>(&self, n: usize, test: F) -> bool
    where
        T: source::Chunk<'source>,
        F: FnOnce(T) -> bool,
    {
        match self.source.read::<T>(self.token_end + n) {
            Some(chunk) => test(chunk),
            None => false,
        }
    }

    /// Bump the position `Lexer` is reading from by `size`.
    #[inline]
    fn bump_unchecked(&mut self, size: usize) {
        debug_assert!(
            self.token_end + size <= self.source.len(),
            "Bumping out of bounds!"
        );

        self.token_end += size;
    }

    /// Reset `token_start` to `token_end`.
    #[inline]
    fn trivia(&mut self) {
        self.extras.on_whitespace();
        self.token_start = self.token_end;
    }

    /// Set the current token to appropriate `#[error]` variant.
    /// Guarantee that `token_end` is at char boundary for `&str`.
    #[inline]
    fn error(&mut self) {
        self.token_end = self.source.find_boundary(self.token_end);
        self.token = Some(Token::ERROR);
    }

    #[inline]
    fn end(&mut self) {
        self.token = None;
    }

    #[inline]
    fn set(&mut self, token: Token) {
        self.token = Some(token);
    }
}
