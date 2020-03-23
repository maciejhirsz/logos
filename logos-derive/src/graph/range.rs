use regex_syntax::hir::ClassUnicodeRange;
use regex_syntax::hir::ClassBytesRange;

use std::cmp::{Ord, Ordering};
use std::ops::Deref;

#[derive(Clone, Copy, PartialOrd, PartialEq, Eq)]
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

impl From<ClassUnicodeRange> for Range {
    fn from(r: ClassUnicodeRange) -> Range {
        Range(r.start() as u8, r.end() as u8)
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