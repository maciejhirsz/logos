use crate::source::{self, Source, WithSource, Slice};
use crate::lexer::Lexer;
use crate::{Logos, Extras};

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    /// Read a chunk at current position.
    fn read<Chunk: source::Chunk<'source>>(&self) -> Option<Chunk>;

    /// Read a chunk at current position offset by `n`.
    fn read_at<Chunk: source::Chunk<'source>>(&self, n: usize) -> Option<Chunk>;

    /// Test a chunk at current position with a closure.
    fn test<T: source::Chunk<'source>, F: FnOnce(T) -> bool>(&self, test: F) -> bool;

    /// Test a chunk at current position offset by `n` with a closure.
    fn test_at<T: source::Chunk<'source>, F: FnOnce(T) -> bool>(&self, n: usize, test: F) -> bool;

    /// Bump the position by `size`.
    fn bump(&mut self, size: usize);

    /// Reset `token_start` to `token_end`.
    fn trivia(&mut self);

    /// Set the current token to appropriate `#[error]` variant.
    /// Guarantee that `token_end` is at char boundary for `&str`.
    fn error(&mut self);
}

// pub trait LexerCallback<Token: Logos, Source> {
//     fn with_lexer(self, lexer: &mut Lexer<Token, Source>);
// }

// impl<'source, T, S, F, B> LexerCallback<T, S> for F
// where
//     S: Source<'source>,
//     T: Logos,
//     F: for<'a> Fn(&'a mut Lexer<T, S>) -> B,
//     B: Bump,
// {
//     #[inline]
//     fn with_lexer(self, lexer: &mut Lexer<T, S>) {
//         (self)(lexer).bump(lexer);
//     }
// }

// pub trait ExtrasCallback<Token: Logos, Source> {
//     fn with_lexer(self, lexer: &mut Lexer<Token, Source>);
// }

// impl<T, S, F> ExtrasCallback<T, S> for F
// where
//     T: Logos,
//     F: Fn(&mut T::Extras),
// {
//     #[inline]
//     fn with_lexer(self, lexer: &mut Lexer<T, S>) {
//         (self)(&mut lexer.extras)
//     }
// }

pub trait Callback<'a, Token: Logos, Source, Arguments> {
    type Return;

    fn call(self, lexer: &'a mut Lexer<Token, Source>) -> Self::Return;
}

impl<'a, 'source, T, S, F, A, B> Callback<'a, T, S, A> for F
where
    T: Logos + WithSource<S>,
    S: Source<'source>,
    F: Fn(A) -> B,
    A: Arguments<'a, T, S>,
    B: Bump,
{
    type Return = B;

    #[inline]
    fn call(self, lexer: &'a mut Lexer<T, S>) -> Self::Return {
        (self)(A::args(lexer))
    }
}

pub trait Arguments<'a, Token: Logos, Source> {
    fn args(lexer: &'a mut Lexer<Token, Source>) -> Self;
}

impl<'a, T, S> Arguments<'a, T, S> for &'a mut Lexer<T, S>
where
    T: Logos,
{
    #[inline]
    fn args(lexer: &'a mut Lexer<T, S>) -> Self {
        lexer
    }
}

impl<'a, 'source: 'a, T, S> Arguments<'a, T, S> for (&'a [u8], &'a mut T::Extras)
where
    T: Logos + WithSource<S>,
    S: Source<'source>,
{
    #[inline]
    fn args(lexer: &'a mut Lexer<T, S>) -> Self {
        (lexer.slice().as_bytes(), &mut lexer.extras)
    }
}

impl<'a, 'source: 'a, T, S> Arguments<'a, T, S> for (&'a [u8], &'a [u8])
where
    T: Logos + WithSource<S>,
    S: Source<'source>,
{
    #[inline]
    fn args(lexer: &'a mut Lexer<T, S>) -> Self {
        (lexer.slice().as_bytes(), lexer.remainder().as_bytes())
    }
}

pub trait Bump {
    fn bump<'source, L: LexerInternal<'source>>(self, lexer: &mut L);
}

impl Bump for () {
    #[inline]
    fn bump<'source, L: LexerInternal<'source>>(self, _: &mut L) {}
}

impl Bump for usize {
    #[inline]
    fn bump<'source, L: LexerInternal<'source>>(self, lexer: &mut L) {
        lexer.bump(self)
    }
}

impl Bump for Option<usize> {
    #[inline]
    fn bump<'source, L: LexerInternal<'source>>(self, lexer: &mut L) {
        match self {
            Some(n) => lexer.bump(n),
            None => lexer.error(),
        }
    }
}