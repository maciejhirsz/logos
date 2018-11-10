use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Repeat(Box<Pattern>),
    Alternative(Vec<Pattern>),
}

#[derive(Debug, Clone, Copy)] pub struct ByteIter<'a>(&'a [u8]);
#[derive(Debug, Clone, Copy)] pub struct RegexIter<'a>(&'a [u8]);

impl<'a> From<&'a str> for ByteIter<'a> {
    fn from(str: &'a str) -> Self {
        ByteIter(str.as_bytes())
    }
}

impl<'a> From<&'a str> for RegexIter<'a> {
    fn from(str: &'a str) -> Self {
        RegexIter(str.as_bytes())
    }
}

impl<'a> Iterator for ByteIter<'a> {
    type Item = Pattern;

    fn next(&mut self) -> Option<Pattern> {
        match (self.0).len() {
            0 => None,
            _ => {
                let byte = self.0[0];
                self.0 = &self.0[1..];

                Some(Pattern::Byte(byte))
            }
        }
    }
}

impl<'a> Iterator for RegexIter<'a> {
    type Item = Pattern;

    fn next(&mut self) -> Option<Pattern> {
        let display = ::std::str::from_utf8(self.0).unwrap();

        let mut read = 0;
        let mut pattern = match (self.0).get(0)? {
            b'[' => {
                let first = *(self.0).get(1).expect("#[regex] Unclosed `[`");
                read += 2;

                assert!(first != b']', "#[regex] Empty `[]` in {}", display);

                let mut patterns = vec![Pattern::Byte(first)];

                loop {
                    match *(self.0).get(read).expect("#[regex] Unclosed `[`") {
                        b']' => {
                            read += 1;
                            break;
                        },
                        b'-' => {
                            read += 1;
                            let last = patterns.pop().unwrap();
                            let from = match last {
                                Pattern::Byte(from) => from,
                                _ => panic!("#[regex] Unexpected `-` in {}", display)
                            };
                            // FIXME: make sure it's a legit character!
                            let to = *(self.0).get(read).unwrap();
                            read += 1;

                            patterns.push(Pattern::Range(from, to));
                        },
                        byte => {
                            read += 1;

                            patterns.push(Pattern::Byte(byte));
                        },
                    }
                }

                if patterns.len() == 1 {
                    patterns.pop().unwrap()
                } else {
                    Pattern::Alternative(patterns)
                }
            },
            byte => {
                read += 1;
                Pattern::Byte(*byte)
            },
        };

        if (self.0).get(read) == Some(&b'*') {
            read += 1;
            pattern = Pattern::Repeat(Box::new(pattern));
        }

        self.0 = &self.0[read..];

        Some(pattern)
    }
}

impl Pattern {
    pub fn is_byte(&self) -> bool {
        match self {
            Pattern::Byte(_) => true,
            _ => false,
        }
    }
}

impl Iterator for Pattern {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let (out, new_self) = match self {
            Pattern::Byte(0) => return None,
            Pattern::Byte(b) => {
                let out = Some(*b);

                *b = 0;

                return out;
            },
            Pattern::Range(from, to) => {
                let out = Some(*from);

                *from = from.saturating_add(1);

                if from < to {
                    return out;
                }

                (out, Pattern::Byte(*to))
            },
            Pattern::Repeat(boxed) => {
                let out = boxed.next();
                let mut new_self = Pattern::Byte(0);

                ::std::mem::swap(&mut new_self, &mut **boxed);

                (out, new_self)
            },
            Pattern::Alternative(alts) => {
                let first = alts.iter_mut().skip_while(|pat| **pat == Pattern::Byte(0)).next()?;

                return first.next();
            },
        };

        *self = new_self;

        out
    }
}

impl PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Pattern) -> Option<Ordering> {
        match (self, other) {
            (&Pattern::Byte(ref byte), &Pattern::Byte(ref other)) => Some(byte.cmp(other)),

            _ => None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pattern_iter_byte() {
        let pattern = Pattern::Byte(b'a');

        assert!("a".bytes().eq(pattern));
    }

    #[test]
    fn pattern_iter_range() {
        let pattern = Pattern::Range(b'a', b'f');

        assert!("abcdef".bytes().eq(pattern));
    }

    #[test]
    fn pattern_iter_alternative() {
        let pattern = Pattern::Alternative(vec![
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert!("_$!".bytes().eq(pattern));
    }

    #[test]
    fn pattern_iter_repeat() {
        let pattern = Pattern::Repeat(Box::new(Pattern::Range(b'a', b'f')));

        assert!("abcdef".bytes().eq(pattern));
    }

    #[test]
    fn pattern_iter_complex() {
        let pattern = Pattern::Alternative(vec![
            Pattern::Repeat(Box::new(Pattern::Range(b'a', b'f'))),
            Pattern::Range(b'0', b'9'),
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert!("abcdef0123456789_$!".bytes().eq(pattern));
    }

    #[test]
    fn regex_number() {
        let regex = RegexIter::from("[1-9][0-9]*");

        assert!(regex.eq([
            Pattern::Range(b'1', b'9'),
            Pattern::Repeat(Box::new(Pattern::Range(b'0', b'9'))),
        ].iter().cloned()));
    }

    #[test]
    fn regex_ident() {
        let regex = RegexIter::from("[a-zA-Z_$][a-zA-Z0-9_$]*");

        assert!(regex.eq([
            Pattern::Alternative(vec![
                Pattern::Range(b'a', b'z'),
                Pattern::Range(b'A', b'Z'),
                Pattern::Byte(b'_'),
                Pattern::Byte(b'$'),
            ]),
            Pattern::Repeat(Box::new(Pattern::Alternative(vec![
                Pattern::Range(b'a', b'z'),
                Pattern::Range(b'A', b'Z'),
                Pattern::Range(b'0', b'9'),
                Pattern::Byte(b'_'),
                Pattern::Byte(b'$'),
            ]))),
        ].iter().cloned()));
    }
}
