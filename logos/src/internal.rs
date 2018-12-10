use crate::source::ByteArray;

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    /// Read the byte at current position.
    fn read(&self) -> u8;

    /// Bump the position by 1 and read the following byte.
    fn next(&mut self) -> u8;

    /// Bump the position by 1.
    fn bump(&mut self, size: usize);

    fn read_bytes<Array>(&self) -> Option<&'source Array>
    where
        Array: ByteArray<'source>;
}
