use crate::source::Chunk;
use crate::Logos;

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    type Token;

    /// Read a chunk at current position.
    fn read<T: Chunk<'source>>(&self) -> Option<T>;

    /// Read a chunk at current position offset by `n`.
    fn read_at<T: Chunk<'source>>(&self, n: usize) -> Option<T>;

    /// Test a chunk at current position with a closure.
    fn test<T: Chunk<'source>, F: FnOnce(T) -> bool>(&self, test: F) -> bool;

    /// Test a chunk at current position offset by `n` with a closure.
    fn test_at<T:Chunk<'source>, F: FnOnce(T) -> bool>(&self, n: usize, test: F) -> bool;

    /// Bump the position by `size`.
    fn bump_unchecked(&mut self, size: usize);

    /// Reset `token_start` to `token_end`.
    fn trivia(&mut self);

    /// Set the current token to appropriate `#[error]` variant.
    /// Guarantee that `token_end` is at char boundary for `&str`.
    fn error(&mut self);

    fn end(&mut self);

    fn set(&mut self, token: Self::Token);
}

pub trait CallbackResult<P> {
    fn construct<'s, F, T: Logos<'s>>(self, constructor: F) -> T
    where
        F: Fn(P) -> T;
}

impl<P> CallbackResult<P> for P {
    #[inline]
    fn construct<'s, F, T: Logos<'s>>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        constructor(self)
    }
}

impl CallbackResult<()> for bool {
    #[inline]
    fn construct<'s, F, T: Logos<'s>>(self, constructor: F) -> T
    where
        F: Fn(()) -> T,
    {
        match self {
            true => constructor(()),
            false => T::ERROR,
        }
    }
}

impl<P> CallbackResult<P> for Option<P> {
    #[inline]
    fn construct<'s, F, T: Logos<'s>>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        match self {
            Some(product) => constructor(product),
            None => T::ERROR,
        }
    }
}

impl<P, E> CallbackResult<P> for Result<P, E> {
    #[inline]
    fn construct<'s, F, T: Logos<'s>>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        match self {
            Ok(product) => constructor(product),
            Err(_) => T::ERROR,
        }
    }
}
