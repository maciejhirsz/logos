use crate::source::Chunk;
use crate::{Filter, FilterResult, Logos, Skip};

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and its methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    type Token: Logos<'source>;

    /// Get the current offset of token_start
    fn offset(&self) -> usize;

    /// Read a chunk
    fn read<T: Chunk<'source>>(&self, offset: usize) -> Option<T>;

    /// Reset `token_start` to `token_end`.
    fn trivia(&mut self);

    /// Guarantee that `token_end` is at char boundary for `&str`.
    /// Called before returning the default error variant.
    fn end_to_boundary(&mut self, offset: usize);

    /// Set `token_end` to an offset.
    fn end(&mut self, offset: usize);
}

//TODO: Seems to me that we are missing a way to return Ok(Token::Uint) or skip matched input,
// similar to Filter<T> but for unit variants.
pub enum CallbackResult<'a, L: Logos<'a>> {
    Emit(L),
    Error(L::Error),
    DefaultError,
    Skip,
}

pub trait CallbackRetVal<'a, P, L: Logos<'a>> {
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(P) -> L;
}

// Field variant implementations

impl<'a, L: Logos<'a>, T> CallbackRetVal<'a, T, L> for T {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(T) -> L,
    {
        CallbackResult::Emit(con(self))
    }
}

impl<'a, L: Logos<'a>, T, E: Into<L::Error>> CallbackRetVal<'a, T, L> for Result<T, E> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Ok(val) => CallbackResult::Emit(con(val)),
            Err(err) => CallbackResult::Error(err.into()),
        }
    }
}

impl<'a, L: Logos<'a>, T> CallbackRetVal<'a, T, L> for Option<T> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Some(val) => CallbackResult::Emit(con(val)),
            None => CallbackResult::DefaultError,
        }
    }
}

impl<'a, L: Logos<'a>, T> CallbackRetVal<'a, T, L> for Filter<T> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Filter::Emit(val) => CallbackResult::Emit(con(val)),
            Filter::Skip => CallbackResult::Skip,
        }
    }
}

impl<'a, L: Logos<'a>, T, E: Into<L::Error>> CallbackRetVal<'a, T, L> for FilterResult<T, E> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            FilterResult::Emit(val) => CallbackResult::Emit(con(val)),
            FilterResult::Skip => CallbackResult::Skip,
            FilterResult::Error(err) => CallbackResult::Error(err.into()),
        }
    }
}

// Unit variant implementations

impl<'a, L: Logos<'a>> CallbackRetVal<'a, (), L> for bool {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            true => CallbackResult::Emit(con(())),
            false => CallbackResult::DefaultError,
        }
    }
}

impl<'a, L: Logos<'a>> CallbackRetVal<'a, (), L> for Skip {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        CallbackResult::Skip
    }
}

impl<'a, L: Logos<'a>, E: Into<L::Error>> CallbackRetVal<'a, (), L> for Result<Skip, E> {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            Ok(Skip) => CallbackResult::Skip,
            Err(err) => CallbackResult::Error(err.into()),
        }
    }
}

// Any token callbacks (only for unit variants due to impl coherency rules)

impl<'a, L: Logos<'a>> CallbackRetVal<'a, (), L> for L {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        CallbackResult::Emit(self)
    }
}

impl<'a, L: Logos<'a>, E: Into<L::Error>> CallbackRetVal<'a, (), L> for Result<L, E> {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            Ok(tok) => CallbackResult::Emit(tok),
            Err(err) => CallbackResult::Error(err.into()),
        }
    }
}

impl<'a, L: Logos<'a>> CallbackRetVal<'a, (), L> for Filter<L> {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            Filter::Emit(tok) => CallbackResult::Emit(tok),
            Filter::Skip => CallbackResult::Skip,
        }
    }
}

impl<'a, L: Logos<'a>, E: Into<L::Error>> CallbackRetVal<'a, (), L> for FilterResult<L, E> {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'a, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            FilterResult::Emit(tok) => CallbackResult::Emit(tok),
            FilterResult::Skip => CallbackResult::Skip,
            FilterResult::Error(err) => CallbackResult::Error(err.into()),
        }
    }
}

pub enum SkipResult<'a, L: Logos<'a>> {
    Skip,
    Error(L::Error),
}

impl<'a, L: Logos<'a>> From<SkipResult<'a, L>> for CallbackResult<'a, L> {
    fn from(value: SkipResult<'a, L>) -> Self {
        match value {
            SkipResult::Skip => CallbackResult::Skip,
            SkipResult::Error(e) => CallbackResult::Error(e),
        }
    }
}

pub trait SkipRetVal<'a, L: Logos<'a>> {
    fn construct(self) -> SkipResult<'a, L>;
}

impl<'a, L: Logos<'a>> SkipRetVal<'a, L> for () {
    #[inline]
    fn construct(self) -> SkipResult<'a, L> {
        SkipResult::Skip
    }
}

impl<'a, L: Logos<'a>> SkipRetVal<'a, L> for Skip {
    #[inline]
    fn construct(self) -> SkipResult<'a, L> {
        SkipResult::Skip
    }
}

impl<'a, L: Logos<'a>, E: Into<L::Error>> SkipRetVal<'a, L> for Result<(), E> {
    #[inline]
    fn construct(self) -> SkipResult<'a, L> {
        match self {
            Ok(()) => SkipResult::Skip,
            Err(err) => SkipResult::Error(err.into()),
        }
    }
}

impl<'a, L: Logos<'a>, E: Into<L::Error>> SkipRetVal<'a, L> for Result<Skip, E> {
    #[inline]
    fn construct(self) -> SkipResult<'a, L> {
        match self {
            Ok(Skip) => SkipResult::Skip,
            Err(err) => SkipResult::Error(err.into()),
        }
    }
}
