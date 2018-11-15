use tree::{Node, Fork};
use syn::Ident;
use regex_syntax::Parser;
use regex_syntax::hir::{self, Hir, HirKind};
use std::cmp::Ordering;
use std::{fmt, mem};

static NO_ZERO_BYTE: &str = "Tokens mustn't include the `0` byte.";

#[derive(Clone, Default)]
pub struct Regex {
    patterns: Vec<Pattern>,
    offset: usize,
}

impl<'a> Node<'a> {
    pub fn from_sequence(source: &str, token: &'a Ident) -> Self {
        let regex = Regex::sequence(source);

        Node::new(regex, token)
    }

    pub fn from_regex(source: &str, token: &'a Ident) -> Self {
        let hir = Parser::new().parse(source).unwrap().into_kind();

        Self::from_hir(hir, token)
    }

    fn from_hir(mut hir: HirKind, token: &'a Ident) -> Self {
        match hir {
            HirKind::Empty => panic!("Empty #[regex] pattern in variant: {}!", token),
            HirKind::Alternation(alternation) => {
                let mut fork = Fork::default();

                for hir in alternation.into_iter().map(Hir::into_kind) {
                    fork.insert(Node::from_hir(hir, token));
                }

                Node::from(fork)
            },
            _ => {
                let mut regex = Regex::default();

                Regex::from_hir_internal(&mut hir, &mut regex.patterns);

                Node::new(regex, token)
            }
        }
    }
}

impl Regex {
    pub fn len(&self) -> usize {
        self.patterns().len()
    }

    #[cfg(test)]
    fn from_regex(source: &str) -> Self {
        let mut hir = Parser::new().parse(source).unwrap().into_kind();
        let mut regex = Regex::default();

        Self::from_hir_internal(&mut hir, &mut regex.patterns);

        regex
    }

    pub fn sequence(source: &str) -> Self {
        Regex {
            patterns: source.bytes().map(|byte| {
                assert!(byte != 0, NO_ZERO_BYTE);

                Pattern::Byte(byte)
            }).collect(),
            offset: 0,
        }
    }

    fn from_hir_internal(hir: &mut HirKind, patterns: &mut Vec<Pattern>) {
        match hir {
            HirKind::Empty => {},
            HirKind::Literal(literal) => {
                use self::hir::Literal;

                match literal {
                    Literal::Unicode(unicode) => {
                        assert!(*unicode != 0 as char, NO_ZERO_BYTE);

                        let mut buf = [0u8; 4];

                        patterns.extend(
                            unicode
                                .encode_utf8(&mut buf)
                                .bytes()
                                .map(Pattern::Byte)
                        );
                    },
                    Literal::Byte(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                };
            },
            HirKind::Class(class) => {
                use self::hir::{Class};

                match class {
                    Class::Unicode(unicode) => {
                        let mut class = unicode
                            .iter()
                            .map(|range| {
                                let (mut start, mut end) = (range.start(), range.end());

                                assert!(end != 0 as char, NO_ZERO_BYTE);

                                static NON_ASCII: &str = "Non-ASCII ranges in #[regex] classes are currently unsupported.";

                                match end as u32 {
                                    0        => panic!("{}", NO_ZERO_BYTE),
                                    0x10FFFF => end = 0xFF as char,
                                    _        => assert!(end.is_ascii(), NON_ASCII),
                                }

                                match start as u32 {
                                    0 => start = 1 as char,
                                    _ => assert!(start.is_ascii(), NON_ASCII),
                                }

                                if start == end {
                                    Pattern::Byte(start as u8)
                                } else {
                                    Pattern::Range(start as u8, end as u8)
                                }
                            })
                            .collect::<Vec<_>>();

                        match class.len() {
                            0 => {},
                            1 => patterns.push(class.remove(0)),
                            _ => patterns.push(Pattern::Class(class)),
                        }
                    },
                    Class::Bytes(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                }
            },
            HirKind::Repetition(repetition) => {
                use self::hir::RepetitionKind;

                // FIXME: needs to take care of the greedy flag!

                let flag = match &repetition.kind {
                    RepetitionKind::ZeroOrMore => RepetitionFlag::ZeroOrMore,
                    RepetitionKind::OneOrMore => RepetitionFlag::OneOrMore,
                    RepetitionKind::ZeroOrOne => panic!("The '?' flag in #[regex] is currently unsupported."),
                    RepetitionKind::Range(_) => panic!("The '{n,m}' repetition in #[regex] is currently unsupported."),
                };
                let mut hir = mem::replace(&mut *repetition.hir, Hir::empty()).into_kind();
                let mut inner = Vec::new();

                Self::from_hir_internal(&mut hir, &mut inner);

                // FIXME: Handle casses when len != 0
                assert!(inner.len() == 1, "FIXME: Make an issue on github if this happens");

                patterns.push(Pattern::Repetition(Box::new(inner.remove(0)), flag));
            },
            HirKind::Concat(concat) => {
                for mut hir in concat.drain(..).map(Hir::into_kind) {
                    Self::from_hir_internal(&mut hir, patterns);
                }
            },
            HirKind::WordBoundary(_) => panic!("Word boundaries in #[regex] are currently unsupported."),
            HirKind::Anchor(_) => panic!("Anchors in #[regex] are currently unsupported."),
            HirKind::Group(group) => {
                let mut hir = mem::replace(&mut *group.hir, Hir::empty()).into_kind();

                Self::from_hir_internal(&mut hir, patterns);
            },

            // Handled on Node::from_regex level
            HirKind::Alternation(_) => return,
        }
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.patterns[self.offset..]
    }

    pub fn first(&self) -> &Pattern {
        self.patterns().get(0).unwrap()
    }

    pub fn is_byte(&self) -> bool {
        self.len() == 1 && self.first().is_byte()
    }

    pub fn match_split(&mut self, other: &mut Regex) -> Option<Regex> {
        let patterns = self.patterns()
                           .iter()
                           .zip(other.patterns())
                           .take_while(|(left, right)| left == right)
                           .map(|(left, _)| left)
                           .cloned()
                           .collect::<Vec<_>>();

        match patterns.len() {
            0 => None,
            len => {
                self.offset += len;
                other.offset += len;

                Some(Regex {
                    patterns,
                    offset: 0
                })
            }
        }
    }
}

impl Iterator for Regex {
    type Item = Pattern;

    fn next(&mut self) -> Option<Pattern> {
        match self.patterns.get_mut(self.offset) {
            Some(&mut Pattern::Repetition(ref pat, ref mut flag)) if *flag == RepetitionFlag::OneOrMore => {
                *flag = RepetitionFlag::ZeroOrMore;

                Some((**pat).clone())
            },
            Some(other) => {
                self.offset += 1;

                Some(other.clone())
            },
            None => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepetitionFlag {
    ZeroOrMore,
    OneOrMore,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
    Repetition(Box<Pattern>, RepetitionFlag),
    Class(Vec<Pattern>),
}

fn format_ascii(byte: u8, f: &mut fmt::Formatter) -> fmt::Result {
    if byte.is_ascii() {
        write!(f, "{:?}", byte as char)
    } else {
        write!(f, "{:?}", byte)
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
            Pattern::Repetition(pattern, flag) => write!(f, "{:?}{:?}", pattern, flag),
            Pattern::Class(class) => class.fmt(f),
        }
    }
}

impl fmt::Debug for RepetitionFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepetitionFlag::ZeroOrMore => f.write_str("*"),
            RepetitionFlag::OneOrMore => f.write_str("+"),
        }
    }
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Regex(")?;

        let mut patterns = self.patterns().iter();

        if let Some(pattern) = patterns.next() {
            pattern.fmt(f)?;

            for pattern in patterns {
                write!(f, ", {:?}", pattern)?;
            }
        }

        f.write_str(")")
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
            Pattern::Repetition(_, flag) => match flag {
                RepetitionFlag::ZeroOrMore => true,
                RepetitionFlag::OneOrMore => true,
            },
            _ => false,
        }
    }

    pub fn is_repeat_plus(&self) -> bool {
        match self {
            Pattern::Repetition(_, flag) => match flag {
                RepetitionFlag::ZeroOrMore => false,
                RepetitionFlag::OneOrMore => true,
            },
            _ => false,
        }
    }

    pub fn weight(&self) -> usize {
        match self {
            Pattern::Byte(_) => 1,
            Pattern::Range(_, _) => 2,
            Pattern::Repetition(pat, _) => pat.weight(),
            Pattern::Class(pats) => pats.iter().map(Self::weight).sum(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Pattern::Byte(_) => 1,
            Pattern::Range(a, b) => (*b as usize).saturating_sub(*a as usize) + 1,
            Pattern::Repetition(pat, _) => pat.len(),
            Pattern::Class(pats) => pats.iter().map(Self::len).sum(),
        }
    }

    pub fn write_bytes(&self, target: &mut Vec<u8>) {
        match self {
            Pattern::Byte(b) => target.push(*b),
            Pattern::Range(a, b) => target.extend(*a..=*b),
            Pattern::Repetition(boxed, _) => boxed.write_bytes(target),
            Pattern::Class(class) => class.iter().for_each(|pat| pat.write_bytes(target)),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len());

        self.write_bytes(&mut bytes);

        bytes
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

        assert_eq!(b"a", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_iter_range() {
        let pattern = Pattern::Range(b'a', b'f');

        assert_eq!(b"abcdef", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_iter_alternative() {
        let pattern = Pattern::Class(vec![
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert_eq!(b"_$!", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_iter_repeat() {
        let pattern = Pattern::Repetition(
            Box::new(Pattern::Range(b'a', b'f')),
            RepetitionFlag::ZeroOrMore
        );

        assert_eq!(b"abcdef", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_iter_complex() {
        let pattern = Pattern::Class(vec![
            Pattern::Repetition(
                Box::new(Pattern::Range(b'a', b'f')),
                RepetitionFlag::ZeroOrMore
            ),
            Pattern::Range(b'0', b'9'),
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert_eq!(b"abcdef0123456789_$!", &pattern.to_bytes()[..]);
    }

    #[test]
    fn regex_number() {
        assert_eq!(
            Regex::from_regex("[1-9][0-9]*").patterns(),
            &[
                Pattern::Range(b'1', b'9'),
                Pattern::Repetition(
                    Box::new(Pattern::Range(b'0', b'9')),
                    RepetitionFlag::ZeroOrMore
                ),
            ]
        );
    }

    #[test]
    fn regex_ident() {
        assert_eq!(
            Regex::from_regex("[a-zA-Z_$][a-zA-Z0-9_$]*").patterns(),
            &[
                Pattern::Class(vec![
                    Pattern::Byte(b'$'),
                    Pattern::Range(b'A', b'Z'),
                    Pattern::Byte(b'_'),
                    Pattern::Range(b'a', b'z'),
                ]),
                Pattern::Repetition(
                    Box::new(Pattern::Class(vec![
                        Pattern::Byte(b'$'),
                        Pattern::Range(b'0', b'9'),
                        Pattern::Range(b'A', b'Z'),
                        Pattern::Byte(b'_'),
                        Pattern::Range(b'a', b'z'),
                    ])),
                    RepetitionFlag::ZeroOrMore
                ),
            ]
        );
    }

    #[test]
    fn regex_hex() {
        assert_eq!(
            Regex::from_regex("0x[0-9a-fA-F]+").patterns(),
            &[
                Pattern::Byte(b'0'),
                Pattern::Byte(b'x'),
                Pattern::Repetition(
                    Box::new(Pattern::Class(vec![
                        Pattern::Range(b'0', b'9'),
                        Pattern::Range(b'A', b'F'),
                        Pattern::Range(b'a', b'f'),
                    ])),
                    RepetitionFlag::OneOrMore
                ),
            ]
        );
    }
}
