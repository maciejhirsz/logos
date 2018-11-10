use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Repeat(Box<Pattern>),
    Alternative(Vec<Pattern>),
}

pub trait Parser {
    fn parse(&[u8]) -> Option<(Pattern, usize)>;
}

pub struct ByteParser;
pub struct RegexParser;

impl Parser for ByteParser {
    fn parse(path: &[u8]) -> Option<(Pattern, usize)> {
        path.get(0).map(|byte| (Pattern::Byte(*byte), 1))
    }
}

impl Parser for RegexParser {
    fn parse(path: &[u8]) -> Option<(Pattern, usize)> {
        let display = ::std::str::from_utf8(path).unwrap();

        let mut read = 0;
        let mut pattern = match path.get(0)? {
            b'[' => {
                let first = *path.get(1).expect("#[regex] Unclosed `[`");
                read += 2;

                assert!(first != b']', "#[regex] Empty `[]` in {}", display);

                let mut patterns = vec![Pattern::Byte(first)];

                loop {
                    match *path.get(read).expect("#[regex] Unclosed `[`") {
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
                            let to = *path.get(read).unwrap();
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

        if path.get(read) == Some(&b'*') {
            read += 1;
            pattern = Pattern::Repeat(Box::new(pattern));
        }

        Some((pattern, read))
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
}
