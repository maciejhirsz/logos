use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Regex(Vec<Pattern>);

impl Regex {
    pub fn from(source: &str) -> Self {
        Regex(RegexIter::from(source).collect())
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.0
    }

    /// Remove and return the `Pattern` matching the first
    /// byte of a string.
    ///
    /// If said `Pattern` is repeating, it won't be removed.
    pub fn first(&mut self) -> Pattern {
        match self.0.first_mut().unwrap() {
            Pattern::Flagged(first, flag) => {
                if *flag == PatternFlag::RepeatPlus {
                    *flag = PatternFlag::Repeat;
                }

                return (**first).clone();
            }
            _ => {},
        }

        self.0.remove(0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternFlag {
    Repeat,
    RepeatPlus,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Flagged(Box<Pattern>, PatternFlag),
    Alternative(Vec<Pattern>),
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::Byte(byte) => (*byte as char).fmt(f),
            Pattern::Range(from, to) => write!(f, "{:?}...{:?}", *from as char, *to as char),
            Pattern::Flagged(pattern, flag) => write!(f, "({:?}){:?}", pattern, flag),
            Pattern::Alternative(alts) => write!(f, "{:?}", alts),
        }
    }
}

impl fmt::Debug for PatternFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PatternFlag::Repeat => f.write_str("*"),
            PatternFlag::RepeatPlus => f.write_str("+"),
        }
    }
}

#[derive(Clone, Copy)] pub struct ByteIter<'a>(&'a [u8]);
#[derive(Clone, Copy)] pub struct RegexIter<'a>(&'a [u8]);

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

        if let Some(flag) = match self.0.get(read).cloned().unwrap_or(0) {
            b'*' => Some(PatternFlag::Repeat),
            b'+' => Some(PatternFlag::RepeatPlus),
            _    => None,
        } {
            read += 1;
            pattern = Pattern::Flagged(Box::new(pattern), flag);
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

    pub fn is_repeat(&self) -> bool {
        match self {
            Pattern::Flagged(_, flag) => match flag {
                PatternFlag::Repeat => true,
                PatternFlag::RepeatPlus => true,
            },
            _ => false,
        }
    }

    // pub fn contains(&self, other: u8) -> bool {
    //     match self {
    //         Pattern::Byte(byte) => *byte == other,
    //         Pattern::Range(from, to) => *from <= other && other <= *to,
    //         Pattern::Repeat(pat) => pat.contains(other),
    //         Pattern::Alternative(alt) => alt.iter().any(|pat| pat.contains(other)),
    //     }
    // }
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
            Pattern::Flagged(boxed, _) => {
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
        let pattern = Pattern::Flagged(
            Box::new(Pattern::Range(b'a', b'f')),
            PatternFlag::Repeat
        );

        assert!("abcdef".bytes().eq(pattern));
    }

    #[test]
    fn pattern_iter_complex() {
        let pattern = Pattern::Alternative(vec![
            Pattern::Flagged(
                Box::new(Pattern::Range(b'a', b'f')),
                PatternFlag::Repeat
            ),
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
            Pattern::Flagged(
                Box::new(Pattern::Range(b'0', b'9')),
                PatternFlag::Repeat
            ),
        ].iter().cloned()));
    }

    #[test]
    fn regex_ident() {
        assert_eq!(
            Regex::from("[a-zA-Z_$][a-zA-Z0-9_$]*").patterns(),
            &[
                Pattern::Alternative(vec![
                    Pattern::Range(b'a', b'z'),
                    Pattern::Range(b'A', b'Z'),
                    Pattern::Byte(b'_'),
                    Pattern::Byte(b'$'),
                ]),
                Pattern::Flagged(
                    Box::new(Pattern::Alternative(vec![
                        Pattern::Range(b'a', b'z'),
                        Pattern::Range(b'A', b'Z'),
                        Pattern::Range(b'0', b'9'),
                        Pattern::Byte(b'_'),
                        Pattern::Byte(b'$'),
                    ])),
                    PatternFlag::Repeat
                ),
            ]
        );
    }

    #[test]
    fn regex_hex() {
        assert_eq!(
            Regex::from("0x[0-9a-fA-F]+").patterns(),
            &[
                Pattern::Byte(b'0'),
                Pattern::Byte(b'x'),
                Pattern::Flagged(
                    Box::new(Pattern::Alternative(vec![
                        Pattern::Range(b'0', b'9'),
                        Pattern::Range(b'a', b'f'),
                        Pattern::Range(b'A', b'F'),
                    ])),
                    PatternFlag::RepeatPlus
                ),
            ]
        );
    }
}
