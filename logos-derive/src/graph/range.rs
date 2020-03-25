use regex_syntax::hir::ClassUnicodeRange;
use regex_syntax::hir::ClassBytesRange;
use utf8_ranges::Utf8Range;

use std::cmp::{Ord, Ordering};

#[derive(Clone, Copy, PartialOrd, PartialEq, Eq, Hash)]
pub struct Range(pub u8, pub u8);

impl From<u8> for Range {
    fn from(byte: u8) -> Range {
        Range(byte, byte)
    }
}

impl From<&u8> for Range {
    fn from(byte: &u8) -> Range {
        Range::from(*byte)
    }
}

impl Iterator for Range {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.0 < self.1 {
            let res = self.0;
            self.0 += 1;

            Some(res)
        } else if self.0 == self.1 {
            let res = self.0;

            // Necessary so that range 0xFF-0xFF doesn't loop forever
            self.0 = 0xFF;
            self.1 = 0x00;

            Some(res)
        } else {
            None
        }
    }
}

impl Ord for Range {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<Utf8Range> for Range {
    fn from(r: Utf8Range) -> Range {
        Range(r.start, r.end)
    }
}

impl From<ClassUnicodeRange> for Range {
    fn from(r: ClassUnicodeRange) -> Range {
        let start = r.start() as u32;
        let end = r.end() as u32;

        if start >= 128 || end >= 128 && end != 0x0010FFFF {
            panic!("Casting non-ascii ClassUnicodeRange to Range")
        }

        Range(start as u8, end as u8)
    }
}

impl From<ClassBytesRange> for Range {
    fn from(r: ClassBytesRange) -> Range {
        Range(r.start(), r.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_iter_one() {
        let byte = Range::from(b'!');
        let collected = byte.into_iter().take(1000).collect::<Vec<_>>();

        assert_eq!(b"!", &collected[..]);
    }

    #[test]
    fn range_iter_few() {
        let byte = Range(b'a', b'd');
        let collected = byte.into_iter().take(1000).collect::<Vec<_>>();

        assert_eq!(b"abcd", &collected[..]);
    }

    #[test]
    fn range_iter_bunds() {
        let byte = Range::from(0xFA..=0xFF);

        let collected = byte.into_iter().take(1000).collect::<Vec<_>>();

        assert_eq!(b"\xFA\xFB\xFC\xFD\xFE\xFF", &collected[..]);
    }
}