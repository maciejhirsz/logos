use crate::source::Chunk;
use crate::{Filter, FilterResult, Lexer, Logos, Skip};

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
    fn error(&mut self, offset: usize);

    /// Set `token_end` to an offset.
    fn end(&mut self, offset: usize);
}

pub enum UnitVariantCallbackResult<E> {
    Emit,
    Error(E),
    DefaultError,
    Skip,
}

impl<E> From<()> for UnitVariantCallbackResult<E> {
    #[inline]
    fn from(_value: ()) -> Self {
        Self::Emit
    }
}

impl<E> From<bool> for UnitVariantCallbackResult<E> {
    #[inline]
    fn from(value: bool) -> Self {
        match value {
            true => Self::Emit,
            false => Self::DefaultError,
        }
    }
}

impl<E> From<Result<(), E>> for UnitVariantCallbackResult<E> {
    #[inline]
    fn from(value: Result<(), E>) -> Self {
        match value {
            Ok(()) => Self::Emit,
            Err(err) => Self::Error(err),
        }
    }
}

impl<E> From<Skip> for UnitVariantCallbackResult<E> {
    #[inline]
    fn from(_value: Skip) -> Self {
        Self::Skip
    }
}

impl<E> From<Result<Skip, E>> for UnitVariantCallbackResult<E> {
    #[inline]
    fn from(value: Result<Skip, E>) -> Self {
        match value {
            Ok(Skip) => Self::Skip,
            Err(err) => Self::Error(err),
        }
    }
}

pub enum FieldVariantCallbackResult<T, E> {
    Emit(T),
    Error(E),
    ErrorDefault,
    Skip,
}

impl <T, E> From<T> for FieldVariantCallbackResult<T, E> {
    #[inline]
    fn from(value: T) -> Self {
        Self::Emit(value)
    }
}

impl<T, E> From<Option<T>> for FieldVariantCallbackResult<T, E> {
    #[inline]
    fn from(value: Option<T>) -> Self {
        match value {
            Some(val) => Self::Emit(val),
            None => Self::ErrorDefault,
        }
    }
}

impl<T, E> From<Result<T, E>> for FieldVariantCallbackResult<T, E> {
    #[inline]
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(val) => Self::Emit(val),
            Err(err) => Self::Error(err),
        }
    }
}

impl<T, E> From<Filter<T>> for FieldVariantCallbackResult<T, E> {
    #[inline]
    fn from(value: Filter<T>) -> Self {
        match value {
            Filter::Emit(val) => Self::Emit(val),
            Filter::Skip => Self::Skip,
        }
    }
}

impl<T, E> From<FilterResult<T, E>> for FieldVariantCallbackResult<T, E> {
    #[inline]
    fn from(value: FilterResult<T, E>) -> Self {
        match value {
            FilterResult::Emit(val) => Self::Emit(val),
            FilterResult::Skip => Self::Skip,
            FilterResult::Error(err) => Self::Error(err),
        }
    }
}


enum SkipCallbackResult<E> {
    Skip,
    Error(E),
    DefaultError,
}

impl<E> From<()> for SkipCallbackResult<E> {
    #[inline]
    fn from(_value: ()) -> Self {
        Self::Skip
    }
}

impl<E> From<Skip> for SkipCallbackResult<E> {
    #[inline]
    fn from(_value: Skip) -> Self {
        Self::Skip
    }
}

impl<E> From<Result<(), E>> for SkipCallbackResult<E> {
    #[inline]
    fn from(value: Result<(), E>) -> Self {
        match value {
            Ok(()) => Self::Skip,
            Err(err) => Self::Error(err),
        }
    }
}

impl<E> From<Result<Skip, E>> for SkipCallbackResult<E> {
    #[inline]
    fn from(value: Result<Skip, E>) -> Self {
        match value {
            Ok(Skip) => Self::Skip,
            Err(err) => Self::Error(err),
        }
    }
}

// TODO: allow callbacks returning Variants themselves

// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for T {
//     #[inline]
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         lex.set(Ok(self))
//     }
// }
//
// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for Result<T, T::Error> {
//     #[inline]
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         match self {
//             Ok(product) => lex.set(Ok(product)),
//             Err(err) => lex.set(Err(err)),
//         }
//     }
// }
//
// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for Filter<T> {
//     #[inline]
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         match self {
//             Filter::Emit(product) => lex.set(Ok(product)),
//             Filter::Skip => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//         }
//     }
// }
//
// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for FilterResult<T, T::Error> {
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         match self {
//             FilterResult::Emit(product) => lex.set(Ok(product)),
//             FilterResult::Skip => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//             FilterResult::Error(err) => lex.set(Err(err)),
//         }
//     }
// }
