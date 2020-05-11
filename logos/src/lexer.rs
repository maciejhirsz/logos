use super::internal::LexerInternal;
use super::Logos;
use crate::source::{self, Source};

use core::mem::ManuallyDrop;

/// Byte range in the source.
pub type Span = core::ops::Range<usize>;

/// `Lexer` is the main struct of the crate that allows you to read through a
/// `Source` and produce tokens for enums implementing the `Logos` trait.
pub struct Lexer<'source, Token: Logos<'source>> {
    source: &'source Token::Source,
    token: ManuallyDrop<Option<Token>>,
    token_start: usize,
    token_end: usize,

    /// Extras associated with the `Token`.
    pub extras: Token::Extras,
}

impl<'source, Token: Logos<'source>> Lexer<'source, Token> {
    /// Create a new `Lexer`.
    ///
    /// Due to type inference, it might be more ergonomic to construct
    /// it by calling [`Token::lexer`](./trait.Logos.html#method.lexer) on any `Token` with derived `Logos`.
    pub fn new(source: &'source Token::Source) -> Self {
        Lexer {
            source,
            token: ManuallyDrop::new(None),
            extras: Default::default(),
            token_start: 0,
            token_end: 0,
        }
    }

    /// Source from which this Lexer is reading tokens.
    #[inline]
    pub fn source(&self) -> &'source Token::Source {
        self.source
    }

    /// Wrap the `Lexer` in a peekable
    /// [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html) that provides a
    /// [`peek`](./struct.PeekableIter.html#method.peek) method,
    /// which allows the next token to be inspected without it being consumed.
    ///
    /// # Example
    ///
    /// ```
    /// use logos::Logos;
    ///
    /// #[derive(Logos, Debug, PartialEq)]
    /// enum Example {
    ///     #[regex(r"[ \n\t\f]+", logos::skip)]
    ///     #[error]
    ///     Error,
    ///
    ///     #[regex("-?[0-9]+", |lex| lex.slice().parse())]
    ///     Integer(i64),
    ///
    ///     #[regex("-?[0-9]+\\.[0-9]+", |lex| lex.slice().parse())]
    ///     Float(f64),
    /// }
    ///
    /// let mut peekable_iter = Example::lexer("42 3.14 -5 f").peekable();
    ///
    /// assert_eq!(Some(&Example::Integer(42)), peekable_iter.peek());
    /// assert_eq!(Some(Example::Integer(42)), peekable_iter.next());
    ///
    /// assert_eq!(Some(&Example::Float(3.14)), peekable_iter.peek());
    /// assert_eq!(Some(&Example::Float(3.14)), peekable_iter.peek());
    /// assert_eq!(Some(Example::Float(3.14)), peekable_iter.next());
    ///
    /// assert_eq!(Some(Example::Integer(-5)), peekable_iter.next());
    ///
    /// assert_eq!(Some(&Example::Error), peekable_iter.peek());
    /// assert_eq!(Some(Example::Error), peekable_iter.next());
    ///
    /// assert_eq!(None, peekable_iter.peek());
    /// assert_eq!(None, peekable_iter.next());
    /// ```
    #[inline]
    pub fn peekable(self) -> PeekableIter<'source, Token> {
        PeekableIter {
            lexer: self,
            peeked: None,
        }
    }

    /// Wrap the `Lexer` in an [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
    /// that produces tuples of `(Token, `[`Span`](./type.Span.html)`)`.
    ///
    /// # Example
    ///
    /// ```
    /// use logos::Logos;
    ///
    /// #[derive(Logos, Debug, PartialEq)]
    /// enum Example {
    ///     #[regex(r"[ \n\t\f]+", logos::skip)]
    ///     #[error]
    ///     Error,
    ///
    ///     #[regex("-?[0-9]+", |lex| lex.slice().parse())]
    ///     Integer(i64),
    ///
    ///     #[regex("-?[0-9]+\\.[0-9]+", |lex| lex.slice().parse())]
    ///     Float(f64),
    /// }
    ///
    /// let tokens: Vec<_> = Example::lexer("42 3.14 -5 f").spanned().collect();
    ///
    /// assert_eq!(
    ///     tokens,
    ///     &[
    ///         (Example::Integer(42), 0..2),
    ///         (Example::Float(3.14), 3..7),
    ///         (Example::Integer(-5), 8..10),
    ///         (Example::Error, 11..12), // 'f' is not a recognized token
    ///     ],
    /// );
    /// ```
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

    /// Get a slice of remaining source, starting at the end of current token.
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
            token: ManuallyDrop::new(None),
            extras: self.extras.into(),
            token_start: self.token_start,
            token_end: self.token_end,
        }
    }

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
        self.token_start = self.token_end;

        Token::lex(self);

        // This basically treats self.token as a temporary field.
        // Since we always immediately return a newly set token here,
        // we don't have to replace it with `None` or manually drop
        // it later.
        unsafe { ManuallyDrop::take(&mut self.token) }
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
        self.lexer.next().map(|token| (token, self.lexer.span()))
    }
}

/// Iterator that allows the next token to be inspected without consuming it.
///
/// Look at [`Lexer::peekable`](./struct.Lexer.html#method.peekable) for documentation.
pub struct PeekableIter<'source, Token: Logos<'source>> {
    lexer: Lexer<'source, Token>,
    peeked: Option<Option<Token>>,
}

impl<'source, Token> PeekableIter<'source, Token>
where
    Token: Logos<'source>,
{
    /// Returns a reference to the next() token without advancing the iterator.
    ///
    /// Look at [`Lexer::peekable`](./struct.Lexer.html#method.peekable) for documentation.
    #[inline]
    pub fn peek(&mut self) -> Option<&Token> {
        let lexer = &mut self.lexer;
        self.peeked.get_or_insert_with(|| lexer.next()).as_ref()
    }

    /// Provides access to the Lexer's [`extras`](./struct.Lexer.html#structfield.extras) field.
    #[inline]
    pub fn extras(&mut self) -> &mut Token::Extras {
        &mut self.lexer.extras
    }

    /// Get the range for the current token in `Source`.
    #[inline]
    pub fn span(&self) -> Span {
        self.lexer.token_start..self.lexer.token_end
    }

    /// Get a string slice of the current token.
    #[inline]
    pub fn slice(&self) -> &'source <Token::Source as Source>::Slice {
        unsafe { self.lexer.source.slice_unchecked(self.lexer.span()) }
    }
}

impl<'source, Token> Iterator for PeekableIter<'source, Token>
where
    Token: Logos<'source>,
{
    type Item = Token;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.peeked.take() {
            Some(token) => token,
            None => self.lexer.next(),
        }
    }

    #[inline]
    fn count(mut self) -> usize {
        match self.peeked.take() {
            Some(None) => 0,
            Some(Some(_)) => 1 + self.lexer.count(),
            None => self.lexer.count(),
        }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self.peeked.take() {
            Some(None) => None,
            Some(v @ Some(_)) if n == 0 => v,
            Some(Some(_)) => self.lexer.nth(n - 1),
            None => self.lexer.nth(n),
        }
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        let peek_opt = match self.peeked.take() {
            Some(None) => return None,
            Some(v) => v,
            None => None,
        };
        self.lexer.last().or(peek_opt)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let peek_len = match self.peeked {
            Some(None) => return (0, Some(0)),
            Some(Some(_)) => 1,
            None => 0,
        };
        let (lo, hi) = self.lexer.size_hint();
        let lo = lo.saturating_add(peek_len);
        let hi = match hi {
            Some(x) => x.checked_add(peek_len),
            None => None,
        };
        (lo, hi)
    }
}

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

    #[inline]
    unsafe fn read_unchecked<Chunk>(&self, n: usize) -> Chunk
    where
        Chunk: source::Chunk<'source>,
    {
        self.source.read_unchecked(self.token_end + n)
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
        self.token_start = self.token_end;
    }

    /// Set the current token to appropriate `#[error]` variant.
    /// Guarantee that `token_end` is at char boundary for `&str`.
    #[inline]
    fn error(&mut self) {
        self.token_end = self.source.find_boundary(self.token_end);
        self.token = ManuallyDrop::new(Some(Token::ERROR));
    }

    #[inline]
    fn end(&mut self) {
        self.token = ManuallyDrop::new(None);
    }

    #[inline]
    fn set(&mut self, token: Token) {
        self.token = ManuallyDrop::new(Some(token));
    }
}
