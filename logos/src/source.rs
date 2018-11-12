use std::ops::Range;

/// Trait for types the `Lexer` can read from.
///
/// Most notably this is implemented for `&str`. It is unlikely you will
/// ever want to use this Trait yourself, unless implementing a new `Source`
/// the `Lexer` can use.
pub trait Source {
    /// Slice of this `Source`, for most types this will be `&str` with
    /// appropriate lifetime.
    type Slice;

    /// Length of the source
    fn len(&self) -> usize;

    /// Read a single byte from source.
    ///
    /// **Implementors of this method must guarantee it to return `0` when
    /// `offset` is set to length of the `Source` (one byte after last)!**
    ///
    /// ```rust
    /// # extern crate logos;
    /// # fn main() {
    /// use logos::Source;
    ///
    /// let foo = "foo";
    ///
    /// unsafe {
    ///     assert_eq!(foo.read(0), b'f');
    ///     assert_eq!(foo.read(1), b'o');
    ///     assert_eq!(foo.read(2), b'o');
    ///     assert_eq!(foo.read(3), 0);
    /// }
    /// # }
    /// ```
    unsafe fn read(&self, offset: usize) -> u8;

    /// Get a slice of the source at given range. This is analogous for
    /// `slice::get_unchecked(range)`.
    ///
    /// ```rust
    /// # extern crate logos;
    /// # fn main() {
    /// use logos::Source;
    ///
    /// let foo = "It was the year when they finally immanentized the Eschaton.";
    ///
    /// unsafe {
    ///     assert_eq!(foo.slice(51..59), "Eschaton");
    /// }
    /// # }
    /// ```
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

/// `Source` implemented on `NulTermStr` from the
/// [`toolshed`](https://crates.io/crates/toolshed) crate.
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
