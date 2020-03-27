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
