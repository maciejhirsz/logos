use crate::source;

/// Trait used by the functions contained in the `Lexicon`.
///
/// # WARNING!
///
/// **This trait, and it's methods, are not meant to be used outside of the
/// code produced by `#[derive(Logos)]` macro.**
pub trait LexerInternal<'source> {
    /// Read a chunk at current position.
    fn read<Chunk: source::Chunk<'source>>(&self) -> Option<Chunk>;

    /// Read a chunk at current position offset by `size`.
    fn lookahead<Chunk: source::Chunk<'source>>(&mut self, size: usize) -> Option<Chunk>;

    /// Bump the position by `size`.
    fn bump(&mut self, size: usize);
}
