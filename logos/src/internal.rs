use crate::source::{self, Source};
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
    fn bump_unchecked(&mut self, size: usize);

    /// Reset `token_start` to `token_end`.
    fn trivia(&mut self);

    /// Set the current token to appropriate `#[error]` variant.
    /// Guarantee that `token_end` is at char boundary for `&str`.
    fn error(&mut self);
}

/// This is a marker trait with no logic.
///
/// Types implementing this trait can be returned directly from
/// a callback constructor, without having to be wrapped in an
/// `Option` or `Result`.
pub trait CallbackProduct {}

pub trait CallbackResult {
    type Product;

    fn construct<F, T: Logos>(self, constructor: F) -> T
    where
        F: Fn(Self::Product) -> T;
}

impl<P: CallbackProduct> CallbackResult for P {
    type Product = P;

    #[inline]
    fn construct<F, T: Logos>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        constructor(self)
    }
}

impl CallbackResult for bool {
    type Product = ();

    #[inline]
    fn construct<F, T: Logos>(self, constructor: F) -> T
    where
        F: Fn(()) -> T,
    {
        match self {
            true => constructor(()),
            false => T::ERROR,
        }
    }
}

impl<P> CallbackResult for Option<P> {
    type Product = P;

    #[inline]
    fn construct<F, T: Logos>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        match self {
            Some(product) => constructor(product),
            None => T::ERROR,
        }
    }
}

impl<P, E> CallbackResult for Result<P, E> {
    type Product = P;

    #[inline]
    fn construct<F, T: Logos>(self, constructor: F) -> T
    where
        F: Fn(P) -> T,
    {
        match self {
            Ok(product) => constructor(product),
            Err(_) => T::ERROR,
        }
    }
}

macro_rules! impl_product {
    ($($t:ty $(: $g:ident)?),*) => {
        $(
            impl $(<$g>)* CallbackProduct for $t {}
        )*
    };
}

impl_product!(
    (), u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, &str, String,
    &[T]: T, Vec<T>: T
);