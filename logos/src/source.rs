use std::ops::Range;
use std::fmt::Debug;

/// Trait for a `Slice` of a `Source` that the `Lexer` can consume.
///
/// Most commonly, those will be the same types:
/// * `&str` slice for `&str` source.
/// * `&[u8]` slice for `&[u8]` source.
pub trait Slice<'source>: Sized + PartialEq + Eq + Debug {
    /// In all implementations we should at least be able to obtain a
    /// slice of bytes as the lowest level common denominator.
    fn as_bytes(&self) -> &'source [u8];
}

impl<'source> Slice<'source> for &'source str {
    fn as_bytes(&self) -> &'source [u8] {
        (*self).as_bytes()
    }
}

impl<'source> Slice<'source> for &'source [u8] {
    fn as_bytes(&self) -> &'source [u8] {
        *self
    }
}

pub trait ByteArray<'source>: Sized {
    const SIZE: usize;

    unsafe fn from_ptr(ptr: *const u8) -> &'source Self {
        &*(ptr as *const Self)
    }
}

impl<'source> ByteArray<'source> for u8 { const SIZE: usize = 1; }
impl<'source> ByteArray<'source> for [u8; 2] { const SIZE: usize = 2; }
impl<'source> ByteArray<'source> for [u8; 3] { const SIZE: usize = 3; }
impl<'source> ByteArray<'source> for [u8; 4] { const SIZE: usize = 4; }
impl<'source> ByteArray<'source> for [u8; 5] { const SIZE: usize = 5; }
impl<'source> ByteArray<'source> for [u8; 6] { const SIZE: usize = 6; }
impl<'source> ByteArray<'source> for [u8; 7] { const SIZE: usize = 7; }
impl<'source> ByteArray<'source> for [u8; 8] { const SIZE: usize = 8; }

/// Trait for types the `Lexer` can read from.
///
/// Most notably this is implemented for `&str`. It is unlikely you will
/// ever want to use this Trait yourself, unless implementing a new `Source`
/// the `Lexer` can use.
pub trait Source<'source> {
    /// A type this `Source` can be sliced into.
    type Slice: self::Slice<'source>;

    /// Length of the source
    fn len(&self) -> usize;

    /// Read a single byte from source.
    ///
    /// **Implementors of this method must guarantee it to return `0` when
    /// `offset` is set to length of the `Source` (one byte after last)!**
    ///
    /// ```rust
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

    /// Read a fixed number of bytes into an array. Returns `None` when reading
    /// out of bounds would occur.
    ///
    /// ```rust
    /// # fn main() {
    /// use logos::Source;
    ///
    /// let foo = "foo";
    ///
    /// unsafe {
    ///     assert_eq!(foo.read_bytes(0), Some(b"foo")); // Option<&[u8; 3]>
    ///     assert_eq!(foo.read_bytes(0), Some(b"fo"));  // Option<&[u8; 2]>
    ///     assert_eq!(foo.read_bytes(2), Some(&b'o'));  // Option<&u8>
    ///     assert_eq!(foo.read_bytes::<[u8; 4]>(0), None);
    ///     assert_eq!(foo.read_bytes::<[u8; 2]>(2), None);
    /// }
    /// # }
    /// ```
    fn read_bytes<Array>(&self, offset: usize) -> Option<&'source Array>
    where
        Array: ByteArray<'source>;

    /// Get a slice of the source at given range. This is analogous to
    /// `slice::get(range)`.
    ///
    /// ```rust
    /// # extern crate logos;
    /// # fn main() {
    /// use logos::Source;
    ///
    /// let foo = "It was the year when they finally immanentized the Eschaton.";
    ///
    /// assert_eq!(Source::slice(&foo, 51..59), Some("Eschaton"));
    /// # }
    /// ```
    fn slice(&self, range: Range<usize>) -> Option<Self::Slice>;

    /// Get a slice of the source at given range. This is analogous to
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
    ///     assert_eq!(Source::slice_unchecked(&foo, 51..59), "Eschaton");
    /// }
    /// # }
    /// ```
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> Self::Slice;
}

impl<'source> Source<'source> for &'source str {
    type Slice = &'source str;

    #[inline]
    fn len(&self) -> usize {
        (*self).len()
    }

    #[inline]
    unsafe fn read(&self, offset: usize) -> u8 {
        debug_assert!(offset <= self.len(), "Reading out founds!");

        match self.as_bytes().get(offset) {
            Some(byte) => *byte,
            None       => 0,
        }
    }

    #[inline]
    fn read_bytes<Array>(&self, offset: usize) -> Option<&'source Array>
    where
        Array: ByteArray<'source>
    {
        if offset + (Array::SIZE - 1) < (*self).len() {
            Some(unsafe { Array::from_ptr((*self).as_ptr().add(offset)) })
        } else {
            None
        }
    }

    #[inline]
    fn slice(&self, range: Range<usize>) -> Option<&'source str> {
        self.get(range)
    }

    #[inline]
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> &'source str {
        debug_assert!(
            range.start <= self.len() && range.end <= self.len(),
            "Reading out of bounds {:?} for {}!", range, self.len()
        );

        self.get_unchecked(range)
    }
}

impl<'source> Source<'source> for &'source [u8] {
    type Slice = &'source [u8];

    #[inline]
    fn len(&self) -> usize {
        (*self).len()
    }

    #[inline]
    unsafe fn read(&self, offset: usize) -> u8 {
        debug_assert!(offset <= self.len(), "Reading out founds!");

        match self.as_bytes().get(offset) {
            Some(byte) => *byte,
            None       => 0,
        }
    }

    #[inline]
    fn read_bytes<Array>(&self, offset: usize) -> Option<&'source Array>
    where
        Array: ByteArray<'source>
    {
        if offset + (Array::SIZE - 1) < (*self).len() {
            Some(unsafe { Array::from_ptr((*self).as_ptr().add(offset)) })
        } else {
            None
        }
    }

    #[inline]
    fn slice(&self, range: Range<usize>) -> Option<&'source [u8]> {
        self.get(range)
    }

    #[inline]
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> &'source [u8] {
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
impl<'source> Source<'source> for toolshed::NulTermStr<'source> {
    type Slice = &'source str;

    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline]
    unsafe fn read(&self, offset: usize) -> u8 {
        debug_assert!(offset <= self.len(), "Reading out founds!");

        self.byte_unchecked(offset)
    }

    #[inline]
    fn read_bytes<Array>(&self, offset: usize) -> Option<&'source Array>
    where
        Array: ByteArray<'source>
    {
        if offset + (Array::SIZE - 1) < (**self).len() {
            Some(unsafe { Array::from_ptr((**self).as_ptr().add(offset)) })
        } else {
            None
        }
    }

    #[inline]
    fn slice(&self, range: Range<usize>) -> Option<&'source str> {
        if range.start <= self.len() && range.end <= self.len() {
            Some(unsafe { self.get_unchecked(range) })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn slice_unchecked(&self, range: Range<usize>) -> &'source str {
        debug_assert!(
            range.start <= self.len() && range.end <= self.len(),
            "Reading out of bounds {:?} for {}!", range, self.len()
        );

        self.get_unchecked(range)
    }
}
