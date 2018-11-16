use tree::{Node, Fork, Branch};
use syn::Ident;
use regex_syntax::Parser;
use regex_syntax::hir::{self, Hir, HirKind};
use std::cmp::Ordering;
use std::fmt;

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

    fn from_hir(hir: HirKind, token: &'a Ident) -> Self {
        match hir {
            HirKind::Empty => panic!("Empty #[regex] pattern in variant: {}!", token),
            HirKind::Alternation(alternation) => {
                let mut fork = Fork::default();

                for hir in alternation.into_iter().map(Hir::into_kind) {
                    fork.insert(Node::from_hir(hir, token));
                }

                Node::from(fork)
            },
            HirKind::Repetition(repetition) => {
                use self::hir::RepetitionKind;

                // FIXME?
                if repetition.greedy == false {
                    panic!("Non greedy parsing in #[regex] is currently unsupported.")
                }

                let (flag, zero) = match repetition.kind {
                    RepetitionKind::ZeroOrOne  => (RepetitionFlag::ZeroOrOne, true),
                    RepetitionKind::ZeroOrMore => (RepetitionFlag::ZeroOrMore, true),
                    RepetitionKind::OneOrMore  => (RepetitionFlag::OneOrMore, false),
                    RepetitionKind::Range(_) => panic!("The '{n,m}' repetition in #[regex] is currently unsupported."),
                };
                let mut node = Node::from_hir(repetition.hir.into_kind(), token);

                node.set_repeat(flag);

                if zero {
                    if let Node::Fork(ref mut fork) = node {
                        fork.insert(Node::Leaf(token));
                    }
                }

                node
            },
            _ => {
                let mut branch = Branch::new(Regex::default(), token);

                Regex::from_hir_internal(hir, &mut branch);

                Node::from(branch)
            }
        }
    }
}

impl<'a> Branch<'a> {
    /// Get the first pattern from the regex on this Branch, removing it unless
    /// it repeats.
    pub fn unshift(&mut self) -> Option<&Pattern> {
        if self.regex.len() == 0 {
            return None;
        }

        let offset = self.regex.offset;

        self.regex.offset += 1;

        if self.regex.len() == 0 {
            self.reset();
        }

        self.regex.patterns.get(offset)
    }

    pub fn consume(&mut self) -> &[Pattern] {
        let offset = self.regex.offset;
        self.regex.offset = self.regex.patterns.len();

        self.reset();

        &self.regex.patterns[offset..]
    }

    fn reset(&mut self) {
        match self.repeat {
            RepetitionFlag::Once => {},
            RepetitionFlag::ZeroOrOne => {
                self.repeat = RepetitionFlag::Once;
            },
            RepetitionFlag::OneOrMore => {
                self.repeat = RepetitionFlag::ZeroOrMore;
                self.regex.offset = 0;
            },
            RepetitionFlag::ZeroOrMore => {
                self.regex.offset = 0;
            }
        }
    }
}

impl Regex {
    pub fn len(&self) -> usize {
        self.patterns().len()
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

    fn from_hir_internal(hir: HirKind, branch: &mut Branch) {
        match hir {
            HirKind::Empty => {},
            HirKind::Literal(literal) => {
                use self::hir::Literal;

                match literal {
                    Literal::Unicode(unicode) => {
                        assert!(unicode != 0 as char, NO_ZERO_BYTE);

                        let mut buf = [0u8; 4];

                        branch.regex.patterns.extend(
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
                            1 => branch.regex.patterns.push(class.remove(0)),
                            _ => branch.regex.patterns.push(Pattern::Class(class)),
                        }
                    },
                    Class::Bytes(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                }
            },
            HirKind::Concat(concat) => {
                for mut hir in concat.into_iter().map(Hir::into_kind) {
                    Self::from_hir_internal(hir, branch);
                }
            },
            HirKind::WordBoundary(_) => panic!("Word boundaries in #[regex] are currently unsupported."),
            HirKind::Anchor(_) => panic!("Anchors in #[regex] are currently unsupported."),
            HirKind::Group(group) => {
                Self::from_hir_internal(group.hir.into_kind(), branch);
            },

            _ => {
                fn insert_at_tail(hir: HirKind, branch: &mut Branch) {
                    let token = match *branch.then {
                        Node::Leaf(ref token) => *token,
                        Node::Branch(ref mut branch) => return insert_at_tail(hir, branch),
                        Node::Fork(ref mut fork) => {
                            for branch in fork.arms.iter_mut() {
                                // FIXME: A way to handle this without cloning?
                                insert_at_tail(hir.clone(), branch);
                            }
                            return;
                        }
                    };

                    let node = Node::from_hir(hir, token);

                    branch.then.replace(node);
                }

                insert_at_tail(hir, branch);
            },
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepetitionFlag {
    Once,
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Byte(u8),
    Range(u8, u8),
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
                f.write_str("...")?;
                format_ascii(*b, f)
            },
            Pattern::Class(class) => class.fmt(f),
        }
    }
}

impl fmt::Debug for RepetitionFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepetitionFlag::Once => f.write_str("n/a"),
            RepetitionFlag::ZeroOrMore => f.write_str("*"),
            RepetitionFlag::OneOrMore => f.write_str("+"),
            RepetitionFlag::ZeroOrOne => f.write_str("?"),
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

    pub fn write_bytes(&self, target: &mut Vec<u8>) {
        match self {
            Pattern::Byte(b) => target.push(*b),
            Pattern::Range(a, b) => target.extend(*a..=*b),
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

            // Lighter first
            _ => self.weight().partial_cmp(&other.weight())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use self::RepetitionFlag::*;

    #[test]
    fn pattern_bytes_byte() {
        let pattern = Pattern::Byte(b'a');

        assert_eq!(b"a", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_bytes_range() {
        let pattern = Pattern::Range(b'a', b'f');

        assert_eq!(b"abcdef", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_bytes_alternative() {
        let pattern = Pattern::Class(vec![
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert_eq!(b"_$!", &pattern.to_bytes()[..]);
    }

    #[test]
    fn pattern_bytes_complex() {
        let pattern = Pattern::Class(vec![
            Pattern::Range(b'a', b'f'),
            Pattern::Range(b'0', b'9'),
            Pattern::Byte(b'_'),
            Pattern::Byte(b'$'),
            Pattern::Byte(b'!'),
        ]);

        assert_eq!(b"abcdef0123456789_$!", &pattern.to_bytes()[..]);
    }

    fn mock_token() -> Ident {
        use proc_macro2::Span;

        Ident::new("mock", Span::call_site())
    }

    fn branch(node: Node) -> Option<Branch> {
        match node {
            Node::Branch(branch) => Some(branch),
            _ => None
        }
    }

    #[test]
    fn branch_regex_number() {
        let token = mock_token();
        let regex = "[1-9][0-9]*";
        let b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'1', b'9')]);
        assert_eq!(b.repeat, Once);

        let b = branch(*b.then).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'0', b'9')]);
        assert_eq!(b.repeat, ZeroOrMore);
    }

    #[test]
    fn regex_ident() {
        let token = mock_token();
        let regex = "[a-zA-Z_$][a-zA-Z0-9_$]*";
        let b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                    Pattern::Byte(b'$'),
                    Pattern::Range(b'A', b'Z'),
                    Pattern::Byte(b'_'),
                    Pattern::Range(b'a', b'z'),
            ])
        ]);
        assert_eq!(b.repeat, Once);

        let b = branch(*b.then).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                Pattern::Byte(b'$'),
                Pattern::Range(b'0', b'9'),
                Pattern::Range(b'A', b'Z'),
                Pattern::Byte(b'_'),
                Pattern::Range(b'a', b'z'),
            ])
        ]);
        assert_eq!(b.repeat, ZeroOrMore);
    }

    #[test]
    fn regex_hex() {
        let token = mock_token();
        let regex = "0x[0-9a-fA-F]+";
        let b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Byte(b'0'),
            Pattern::Byte(b'x'),
        ]);
        assert_eq!(b.repeat, Once);

        let b = branch(*b.then).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                Pattern::Range(b'0', b'9'),
                Pattern::Range(b'A', b'F'),
                Pattern::Range(b'a', b'f'),
            ])
        ]);
        assert_eq!(b.repeat, OneOrMore);
    }

    #[test]
    fn branch_unshift() {
        let token = mock_token();
        let regex = "abc";
        let mut b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Byte(b'a'),
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'a')));
        assert_eq!(b.regex.patterns(), &[
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'b')));
        assert_eq!(b.regex.patterns(), &[
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'c')));
        assert_eq!(b.regex.patterns(), &[]);
        assert_eq!(b.unshift(), None);
    }

    #[test]
    fn branch_unshift_repeat() {
        let token = mock_token();
        let regex = "a+";
        let mut b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Byte(b'a')]);
        assert_eq!(b.repeat, OneOrMore);

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'a')));
        assert_eq!(b.regex.patterns(), &[Pattern::Byte(b'a')]);
        assert_eq!(b.repeat, ZeroOrMore);

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'a')));
        assert_eq!(b.regex.patterns(), &[Pattern::Byte(b'a')]);
        assert_eq!(b.repeat, ZeroOrMore);
    }

    #[test]
    fn branch_unshift_repeat_group() {
        let token = mock_token();
        let regex = "(abc)*";
        let mut b = branch(Node::from_regex(regex, &token)).unwrap();

        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'a')));
        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'b')));
        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'c')));
        assert_eq!(b.unshift(), Some(&Pattern::Byte(b'a')));
    }
}
