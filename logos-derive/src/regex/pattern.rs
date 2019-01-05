use std::cmp::Ordering;
use std::fmt;

use crate::util::{MergeAscending, DiffAscending};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Class(Vec<Pattern>),
}

impl Pattern {
    pub fn new<I>(bytes: I) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        let mut bytes = bytes.into_iter();
        let mut first = bytes.next().expect("Internal Error: Emtpy Pattern");
        let mut last  = first;
        let mut class = Vec::new();

        let mut push = |first, last| {
            if first == last {
                class.push(Pattern::Byte(first));
            } else {
                class.push(Pattern::Range(first, last));
            }
        };

        for byte in bytes {
            if byte == last + 1 {
                last = byte;

                continue;
            }

            push(first, last);

            first = byte;
            last = byte;
        }

        // Handle last values
        push(first, last);

        if class.len() == 1 {
            class.remove(0)
        } else {
            Pattern::Class(class)
        }
    }

    pub fn bytes(&self) -> Bytes {
        let mut bytes = Bytes {
            buf: [0; 256],
            len: 0,
            index: 0,
        };

        bytes.len = self.write_bytes(&mut bytes.buf) as u8;

        bytes
    }

    pub fn is_byte(&self) -> bool {
        match self {
            Pattern::Byte(_) => true,
            _ => false,
        }
    }

    pub fn weight(&self) -> usize {
        match self {
            Pattern::Byte(_) => 1,
            Pattern::Range(_, _) => 2,
            Pattern::Class(pats) => pats.iter().map(Self::weight).sum(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Pattern::Byte(_) => 1,
            Pattern::Range(a, b) => (*b as usize).saturating_sub(*a as usize) + 1,
            Pattern::Class(pats) => pats.iter().map(Self::len).sum(),
        }
    }

    pub fn intersect(&self, other: &Pattern) -> Option<Pattern> {
        if self == other {
            return None;
        }

        if self.contains(other) {
            Some(other.clone())
        } else if other.contains(self) {
            Some(self.clone())
        } else {
            None
        }
    }

    pub fn combine(&mut self, other: &Pattern) {
        *self = Pattern::new(MergeAscending::new(&*self, other));
    }

    pub fn subtract(&mut self, other: &Pattern) {
        *self = Pattern::new(DiffAscending::new(&*self, other));
    }

    pub fn negate(&self) -> Pattern {
        Pattern::new(DiffAscending::new(0..=255, &*self))
    }

    pub fn contains(&self, other: &Pattern) -> bool {
        let mut buffer = [0; 256];
        let offset = self.write_bytes(&mut buffer);
        let bytes = &buffer[..offset];

        other.bytes().all(|byte| bytes.binary_search(&byte).is_ok())
    }

    fn write_bytes(&self, mut target: &mut [u8]) -> usize {
        let len = self.len();

        match self {
            Pattern::Byte(b) => {
                target[0] = *b;
            },
            Pattern::Range(a, b) => {
                for (index, byte) in (*a..=*b).enumerate() {
                    target[index] = byte;
                }
            }
            Pattern::Class(class) => {
                for pat in class.iter() {
                    let len = pat.write_bytes(target);

                    target = &mut target[len..];
                }
            },
        }

        len
    }
}

/// Iterator of all the bytes within the `Pattern`
pub struct Bytes {
    buf: [u8; 256],
    len: u8,
    index: u8,
}

impl Iterator for Bytes {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.index < self.len {
            let byte = self.buf[self.index as usize];
            self.index += 1;
            Some(byte)
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Pattern {
    type Item = u8;
    type IntoIter = Bytes;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes()
    }
}

impl PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Pattern) -> Option<Ordering> {
        match (self, other) {
            (&Pattern::Byte(ref byte), &Pattern::Byte(ref other)) => Some(byte.cmp(other)),

            // Shorter first
            _ => match self.len().partial_cmp(&other.len()) {
                // Equal length != equal patterns, so let's not do that!
                Some(Ordering::Equal) => None,
                ordering              => ordering
            }
        }
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::Byte(byte) => format_ascii(*byte, f),
            Pattern::Range(a, b) => {
                format_ascii(*a, f)?;
                f.write_str("-")?;
                format_ascii(*b, f)
            },
            Pattern::Class(class) => {
                f.write_str("[")?;
                for pat in class.iter() {
                    write!(f, "{:?}", pat)?;
                }
                f.write_str("]")
            },
        }
    }
}

fn format_ascii(byte: u8, f: &mut fmt::Formatter) -> fmt::Result {
    if byte >= 0x20 && byte <= 127 {
        write!(f, "{}", byte as char)
    } else {
        write!(f, "{:02X?}", byte)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_iter_byte() {
        let pattern = Pattern::Byte(b'a');

        assert!(pattern.bytes().eq("a".bytes()));
    }

    #[test]
    fn bytes_iter_range() {
        let pattern = Pattern::Range(b'a', b'f');

        assert!(pattern.bytes().eq("abcdef".bytes()));
    }

    #[test]
    fn bytes_iter_alternative() {
        let pattern = Pattern::Class(vec![
            Pattern::Byte(b'!'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'_'),
        ]);

        assert!(pattern.bytes().eq("!$_".bytes()));
    }

    #[test]
    fn bytes_iter_complex() {
        let pattern = Pattern::Class(vec![
            Pattern::Byte(b'!'),
            Pattern::Byte(b'$'),
            Pattern::Range(b'0', b'9'),
            Pattern::Byte(b'_'),
            Pattern::Range(b'a', b'f'),
        ]);

        assert!(pattern.bytes().eq("!$0123456789_abcdef".bytes()));
    }

    #[test]
    fn from_iterator() {
        let pattern = Pattern::new("!$0123456789_abcdef".bytes());

        let expected = Pattern::Class(vec![
            Pattern::Byte(b'!'),
            Pattern::Byte(b'$'),
            Pattern::Range(b'0', b'9'),
            Pattern::Byte(b'_'),
            Pattern::Range(b'a', b'f'),
        ]);

        assert_eq!(pattern, expected);
    }


    #[test]
    fn combine() {
        let mut pattern = Pattern::Byte(0);

        pattern.combine(&Pattern::Range(b'A', b'Z'));
        pattern.combine(&Pattern::Range(b'0', b'9'));

        let expected = Pattern::Class(vec![
            Pattern::Byte(0),
            Pattern::Range(b'0', b'9'),
            Pattern::Range(b'A', b'Z'),
        ]);

        assert_eq!(pattern, expected);
    }

    #[test]
    fn subtract() {
        let mut pattern = Pattern::Range(b'0', b'9');

        pattern.subtract(&Pattern::Range(b'1', b'3'));
        pattern.subtract(&Pattern::Byte(b'7'));

        let expected = Pattern::Class(vec![
            Pattern::Byte(b'0'),
            Pattern::Range(b'4', b'6'),
            Pattern::Range(b'8', b'9'),
        ]);

        assert_eq!(pattern, expected);
    }

    #[test]
    fn overlapping() {
        let mut pattern = Pattern::Class(vec![
            Pattern::Range(b'0', b'9'),
            Pattern::Range(b'A', b'Z'),
        ]);

        pattern.subtract(&Pattern::Range(b'8', b'E'));

        let expected = Pattern::Class(vec![
            Pattern::Range(b'0', b'7'),
            Pattern::Range(b'F', b'Z'),
        ]);

        assert_eq!(pattern, expected);
    }
}
