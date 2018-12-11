use crate::source::ByteArray;

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    /// Read the byte(s) at current position.
    fn read<Array: ByteArray<'source>>(&self) -> Option<Array>;

    /// Bump the position by 1 and read the following byte.
    fn next(&mut self) -> Option<u8>;

    /// Bump the position by `size`.
    fn bump(&mut self, size: usize);
}
