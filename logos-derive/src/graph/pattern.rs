#[cfg(test)]
#[macro_export]
macro_rules! pat {
    ($($r:expr),*) => {vec![$($r.into()),*]};
}

pub type Pattern = Vec<Range>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Range(pub u8, pub u8);

impl From<u8> for Range {
    fn from(byte: u8) -> Range {
        Range(byte, byte)
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