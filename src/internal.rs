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

// pub trait CallbackResult<'s, P, T: Logos<'s>> {
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T;
// }
//
// pub trait SkipCallbackResult<'s, T: Logos<'s>>
// where
//     Self: Sized,
// {
//     fn into_result(self) -> Result<Skip, T::Error>;
//     fn construct_skip(self, lex: &mut Lexer<'s, T>) {
//         match self.into_result() {
//             Ok(Skip) => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//             Err(e) => lex.set(Err(e)),
//         }
//     }
// }
//
// impl<'s, P, T: Logos<'s>> CallbackResult<'s, P, T> for P {
//     #[inline]
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T,
//     {
//         lex.set(Ok(c(self)))
//     }
// }
//
// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for bool {
//     #[inline]
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         match self {
//             true => lex.set(Ok(c(()))),
//             false => lex.set(Err(T::Error::default())),
//         }
//     }
// }
//
// impl<'s, P, T: Logos<'s>> CallbackResult<'s, P, T> for Option<P> {
//     #[inline]
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T,
//     {
//         match self {
//             Some(product) => lex.set(Ok(c(product))),
//             None => lex.set(Err(T::Error::default())),
//         }
//     }
// }
//
// impl<'s, P, E, T: Logos<'s>> CallbackResult<'s, P, T> for Result<P, E>
// where
//     E: Into<T::Error>,
// {
//     #[inline]
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T,
//     {
//         match self {
//             Ok(product) => lex.set(Ok(c(product))),
//             Err(err) => lex.set(Err(err.into())),
//         }
//     }
// }
//
// impl<'s, T: Logos<'s>> CallbackResult<'s, (), T> for Skip {
//     #[inline]
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         lex.trivia();
//         T::lex(lex);
//     }
// }
//
// impl<'s, E, T: Logos<'s>> CallbackResult<'s, (), T> for Result<Skip, E>
// where
//     E: Into<T::Error>,
// {
//     #[inline]
//     fn construct<Constructor>(self, _: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(()) -> T,
//     {
//         match self {
//             Ok(_) => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//             Err(err) => lex.set(Err(err.into())),
//         }
//     }
// }
//
// impl<'s, P, T: Logos<'s>> CallbackResult<'s, P, T> for Filter<P> {
//     #[inline]
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T,
//     {
//         match self {
//             Filter::Emit(product) => lex.set(Ok(c(product))),
//             Filter::Skip => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//         }
//     }
// }
//
// impl<'s, P, E, T: Logos<'s>> CallbackResult<'s, P, T> for FilterResult<P, E>
// where
//     E: Into<T::Error>,
// {
//     fn construct<Constructor>(self, c: Constructor, lex: &mut Lexer<'s, T>)
//     where
//         Constructor: Fn(P) -> T,
//     {
//         match self {
//             FilterResult::Emit(product) => lex.set(Ok(c(product))),
//             FilterResult::Skip => {
//                 lex.trivia();
//                 T::lex(lex);
//             }
//             FilterResult::Error(err) => lex.set(Err(err.into())),
//         }
//     }
// }
//
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
//
// impl<'s, T: Logos<'s>> SkipCallbackResult<'s, T> for () {
//     fn into_result(self) -> Result<Skip, T::Error> {
//         Ok(Skip)
//     }
// }
//
// impl<'s, T: Logos<'s>> SkipCallbackResult<'s, T> for Skip {
//     fn into_result(self) -> Result<Skip, T::Error> {
//         Ok(self)
//     }
// }
//
// impl<'s, T: Logos<'s>, E> SkipCallbackResult<'s, T> for Result<(), E>
// where
//     E: Into<T::Error>,
// {
//     fn into_result(self) -> Result<Skip, T::Error> {
//         match self {
//             Ok(_) => Ok(Skip),
//             Err(e) => Err(e.into()),
//         }
//     }
// }
//
// impl<'s, T: Logos<'s>, E> SkipCallbackResult<'s, T> for Result<Skip, E>
// where
//     E: Into<T::Error>,
// {
//     fn into_result(self) -> Result<Skip, T::Error> {
//         match self {
//             Ok(skip) => Ok(skip),
//             Err(e) => Err(e.into()),
//         }
//     }
// }
