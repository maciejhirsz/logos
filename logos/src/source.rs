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

pub trait Chunk<'source>: Sized + Copy {
    const SIZE: usize;

    unsafe fn from_ptr(ptr: *const u8) -> Self;
}

pub trait Split<Target> {
    type Remainder;

    fn split(self) -> (Target, Self::Remainder);
}

impl<'source> Chunk<'source> for u8 {
    const SIZE: usize = 1;

    #[inline]
    unsafe fn from_ptr(ptr: *const u8) -> Self {
        *ptr
    }
}

macro_rules! impl_array {
    (@byte $size:expr, 1) => (
        impl<'source> Split<u8> for &'source [u8; $size] {
            type Remainder = &'source [u8; $size - 1];

            #[inline]
            fn split(self) -> (u8, &'source [u8; $size - 1]) {
                unsafe {(
                    self[0],
                    Chunk::from_ptr((self as *const u8).add(1)),
                )}
            }
        }
    );

    (@byte $size:expr, $ignore:tt) => ();

    ($($size:expr => ( $( $split:tt ),* ))*) => ($(
        impl<'source> Chunk<'source> for &'source [u8; $size] {
            const SIZE: usize = $size;

            #[inline]
            unsafe fn from_ptr(ptr: *const u8) -> Self {
                &*(ptr as *const [u8; $size])
            }
        }

        $(
            impl_array! { @byte $size, $split }

            impl<'source> Split<&'source [u8; $split]> for &'source [u8; $size] {
                type Remainder = &'source [u8; $size - $split];

                #[inline]
                fn split(self) -> (&'source [u8; $split], &'source [u8; $size - $split]) {
                    unsafe {(
                        Chunk::from_ptr(self as *const u8),
                        Chunk::from_ptr((self as *const u8).add($split)),
                    )}
                }
            }
        )*
    )*);
}

impl_array! {
    1  => ()
    2  => (1)
    3  => (1, 2)
    4  => (1, 2, 3)
    5  => (1, 2, 3, 4)
    6  => (1, 2, 3, 4, 5)
    7  => (1, 2, 3, 4, 5, 6)
    8  => (1, 2, 3, 4, 5, 6, 7)
    9  => (1, 2, 3, 4, 5, 6, 7, 8)
    10 => (1, 2, 3, 4, 5, 6, 7, 8, 9)
    11 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
    12 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
    13 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
    14 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)
    15 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14)
    16 => (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)
}

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
    ///     assert_eq!(foo.read_bytes(2), Some(b'o'));   // Option<u8>
    ///     assert_eq!(foo.read_bytes::<&[u8; 4]>(0), None);
    ///     assert_eq!(foo.read_bytes::<&[u8; 2]>(2), None);
    /// }
    /// # }
    /// ```
    fn read_bytes<Chunk>(&self, offset: usize) -> Option<Chunk>
    where
        Chunk: self::Chunk<'source>;

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
    fn read_bytes<Chunk>(&self, offset: usize) -> Option<Chunk>
    where
        Chunk: self::Chunk<'source>
    {
        if offset + (Chunk::SIZE - 1) < (*self).len() {
            Some(unsafe { Chunk::from_ptr((*self).as_ptr().add(offset)) })
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
    fn read_bytes<Chunk>(&self, offset: usize) -> Option<Chunk>
    where
        Chunk: self::Chunk<'source>
    {
        if offset + (Chunk::SIZE - 1) < (*self).len() {
            Some(unsafe { Chunk::from_ptr((*self).as_ptr().add(offset)) })
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
