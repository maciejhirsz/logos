#![warn(missing_docs)]

#[cfg(feature = "nul_term_source")]
extern crate toolshed;

mod lexer;

use std::ops::Range;

pub use lexer::{Lexer, LexerInternal, Lexicon, Extras};

/// Trait implemented for an enum representing all tokens. You should never have
/// to implement it manually, use the `#[derive(Logos)]` attribute on your enum.
pub trait Logos: Sized {
    /// Associated `Extras` for the particular lexer. Those can handle things that
    /// aren't necessarily tokens, such as comments or Automatic Semicolon Insertion
    /// in JavaScript.
    type Extras: self::Extras;

    /// `SIZE` is simply a number of possible variants of the `Logos` enum. The
    /// `derive` macro will make sure that all variants don't hold values larger
    /// or equal to `SIZE`.
    ///
    /// This can be extremely useful for creating `Logos` Lookup Tables.
    const SIZE: usize;

    /// Helper const pointing to the variant marked as #[error].
    const ERROR: Self;

    /// Returns a lookup table for the `Lexer`
    fn lexicon<Lexer: LexerInternal<Self>>() -> Lexicon<Lexer>;

    /// Create a new instance of a `Lexer` that will produce tokens implementing
    /// this `Logos`.
    fn lexer<S: Source>(source: S) -> Lexer<Self, S> {
        Lexer::new(source)
    }
}

/// Trait for types the `Lexer` can read from.
pub trait Source {
    type Slice;

    /// Length of the source
    fn len(&self) -> usize;

    /// Read a single byte from source.
    ///
    /// **Implementors of this method must guarantee it to return `0` when
    /// `offset` is set to length of the `Source` (one byte after last)!**
    unsafe fn read(&self, offset: usize) -> u8;

    /// Get a slice of the source at given range. This is analogous for
    /// `slice::get_unchecked(range)`.
    unsafe fn slice(&self, range: Range<usize>) -> Self::Slice;
}

impl<'source> Source for &'source str {
    type Slice = &'source str;

    fn len(&self) -> usize {
        (*self).len()
    }

    unsafe fn read(&self, offset: usize) -> u8 {
        debug_assert!(offset <= self.len(), "Reading out founds!");

        match self.as_bytes().get(offset) {
            Some(byte) => *byte,
            None       => 0,
        }
    }

    unsafe fn slice(&self, range: Range<usize>) -> Self::Slice {
        debug_assert!(
            range.start <= self.len() && range.end <= self.len(),
            "Reading out of bounds {:?} for {}!", range, self.len()
        );

        self.get_unchecked(range)
    }
}

/// `Source` implemented on `NulTermStr` from the `toolshed` crate.
///
/// **This requires the `"nul_term_source"` feature to be enabled.**
#[cfg(feature = "nul_term_source")]
impl<'source> Source for toolshed::NulTermStr<'source> {
    type Slice = &'source str;

    fn len(&self) -> usize {
        (**self).len()
    }

    unsafe fn read(&self, offset: usize) -> u8 {
        debug_assert!(offset <= self.len(), "Reading out founds!");

        self.byte_unchecked(offset)
    }

    unsafe fn slice(&self, range: Range<usize>) -> Self::Slice {
        debug_assert!(
            range.start <= self.len() && range.end <= self.len(),
            "Reading out of bounds {:?} for {}!", range, self.len()
        );

        self.get_unchecked(range)
    }
}
