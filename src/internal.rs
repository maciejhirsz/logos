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
pub enum CallbackResult<'source, L: Logos<'source>> {
    Emit(L),
    Error(L::Error),
    DefaultError,
    Skip,
}

pub trait CallbackRetVal<'source, P, L: Logos<'source>> {
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(P) -> L;
}

// Field variant implementations

impl<'source, L: Logos<'source>, T> CallbackRetVal<'source, T, L> for T {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(T) -> L,
    {
        CallbackResult::Emit(con(self))
    }
}

impl<'source, L: Logos<'source>, T, E: Into<L::Error>> CallbackRetVal<'source, T, L>
    for Result<T, E>
{
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Ok(val) => CallbackResult::Emit(con(val)),
            Err(err) => CallbackResult::Error(err.into()),
        }
    }
}

impl<'source, L: Logos<'source>, T> CallbackRetVal<'source, T, L> for Option<T> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Some(val) => CallbackResult::Emit(con(val)),
            None => CallbackResult::DefaultError,
        }
    }
}

impl<'source, L: Logos<'source>, T> CallbackRetVal<'source, T, L> for Filter<T> {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(T) -> L,
    {
        match self {
            Filter::Emit(val) => CallbackResult::Emit(con(val)),
            Filter::Skip => CallbackResult::Skip,
        }
    }
}

impl<'source, L: Logos<'source>, T, E: Into<L::Error>> CallbackRetVal<'source, T, L>
    for FilterResult<T, E>
{
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
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

impl<'source, L: Logos<'source>> CallbackRetVal<'source, (), L> for bool {
    #[inline]
    fn construct<C>(self, con: C) -> CallbackResult<'source, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            true => CallbackResult::Emit(con(())),
            false => CallbackResult::DefaultError,
        }
    }
}

impl<'source, L: Logos<'source>> CallbackRetVal<'source, (), L> for Skip {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
    where
        C: Fn(()) -> L,
    {
        CallbackResult::Skip
    }
}

impl<'source, L: Logos<'source>, E: Into<L::Error>> CallbackRetVal<'source, (), L>
    for Result<Skip, E>
{
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
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

impl<'source, L: Logos<'source>> CallbackRetVal<'source, (), L> for L {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
    where
        C: Fn(()) -> L,
    {
        CallbackResult::Emit(self)
    }
}

impl<'source, L: Logos<'source>, E: Into<L::Error>> CallbackRetVal<'source, (), L>
    for Result<L, E>
{
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            Ok(tok) => CallbackResult::Emit(tok),
            Err(err) => CallbackResult::Error(err.into()),
        }
    }
}

impl<'source, L: Logos<'source>> CallbackRetVal<'source, (), L> for Filter<L> {
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
    where
        C: Fn(()) -> L,
    {
        match self {
            Filter::Emit(tok) => CallbackResult::Emit(tok),
            Filter::Skip => CallbackResult::Skip,
        }
    }
}

impl<'source, L: Logos<'source>, E: Into<L::Error>> CallbackRetVal<'source, (), L>
    for FilterResult<L, E>
{
    #[inline]
    fn construct<C>(self, _con: C) -> CallbackResult<'source, L>
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

pub enum SkipResult<'source, L: Logos<'source>> {
    Skip,
    Error(L::Error),
}

impl<'source, L: Logos<'source>> From<SkipResult<'source, L>> for CallbackResult<'source, L> {
    fn from(value: SkipResult<'source, L>) -> Self {
        match value {
            SkipResult::Skip => CallbackResult::Skip,
            SkipResult::Error(e) => CallbackResult::Error(e),
        }
    }
}

pub trait SkipRetVal<'source, L: Logos<'source>> {
    fn construct(self) -> SkipResult<'source, L>;
}

impl<'source, L: Logos<'source>> SkipRetVal<'source, L> for () {
    #[inline]
    fn construct(self) -> SkipResult<'source, L> {
        SkipResult::Skip
    }
}

impl<'source, L: Logos<'source>> SkipRetVal<'source, L> for Skip {
    #[inline]
    fn construct(self) -> SkipResult<'source, L> {
        SkipResult::Skip
    }
}

impl<'source, L: Logos<'source>, E: Into<L::Error>> SkipRetVal<'source, L> for Result<(), E> {
    #[inline]
    fn construct(self) -> SkipResult<'source, L> {
        match self {
            Ok(()) => SkipResult::Skip,
            Err(err) => SkipResult::Error(err.into()),
        }
    }
}

impl<'source, L: Logos<'source>, E: Into<L::Error>> SkipRetVal<'source, L> for Result<Skip, E> {
    #[inline]
    fn construct(self) -> SkipResult<'source, L> {
        match self {
            Ok(Skip) => SkipResult::Skip,
            Err(err) => SkipResult::Error(err.into()),
        }
    }
}
