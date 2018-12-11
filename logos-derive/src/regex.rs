use utf8_ranges::{Utf8Sequences, Utf8Sequence, Utf8Range};
use regex_syntax::hir::{self, Hir, HirKind, Class};
use regex_syntax::Parser;
use std::cmp::Ordering;
use std::fmt;

use crate::tree::{Node, Fork, ForkKind, Branch, Leaf};

#[derive(Clone, Default)]
pub struct Regex {
    patterns: Vec<Pattern>,
    offset: usize,
}

impl PartialEq for Regex {
    fn eq(&self, other: &Regex) -> bool {
        self.patterns() == other.patterns()
    }
}

impl<'a> Node<'a> {
    pub fn from_sequence(source: &str, leaf: Leaf<'a>) -> Self {
        let regex = Regex::sequence(source);

        if regex.len() == 0 {
            panic!("Empty #[token] string in variant: {}!", leaf.token);
        }

        Node::new(regex, leaf)
    }

    pub fn from_regex(source: &str, leaf: Leaf<'a>) -> Self {
        let hir = match Parser::new().parse(source) {
            Ok(hir) => hir.into_kind(),
            Err(err) => panic!("Unable to parse #[regex] for variant: {}!\n\n{:#?}", leaf.token, err),
        };

        let mut node = Self::from_hir(hir).expect("Unable to produce a valid tree for #[regex]");
        let leaf = Node::Leaf(leaf);

        node.chain(&leaf);

        node
    }

    fn from_hir(hir: HirKind) -> Option<Self> {
        match hir {
            HirKind::Empty => None,
            HirKind::Alternation(alternation) => {
                let mut fork = Fork::default();

                for hir in alternation.into_iter().map(Hir::into_kind) {
                    if let Some(node) = Node::from_hir(hir) {
                        fork.insert(node);
                    } else {
                        fork.kind = ForkKind::Maybe;
                    }
                }

                Some(Node::from(fork))
            },
            HirKind::Concat(concat) => {
                let mut concat = concat.into_iter().map(Hir::into_kind).collect::<Vec<_>>();
                let mut nodes = vec![];
                let mut read = 0;

                while concat.len() != read {
                    let mut regex = Regex::default();

                    let count = concat[read..].iter().take_while(|hir| {
                        Regex::from_hir_internal(hir, &mut regex)
                    }).count();

                    if count != 0 {
                        nodes.push(Branch::new(regex).into());
                        read += count;
                    } else {
                        if let Some(node) = Node::from_hir(concat.remove(read)) {
                            nodes.push(node);
                        }
                    }
                }

                let mut node = nodes.pop()?;

                for mut n in nodes.into_iter().rev() {
                    n.chain(&node);

                    node = n;
                }

                Some(node)
            },
            HirKind::Repetition(repetition) => {
                use self::hir::RepetitionKind;

                // FIXME?
                if repetition.greedy == false {
                    panic!("Non-greedy parsing in #[regex] is currently unsupported.")
                }

                let flag = match repetition.kind {
                    RepetitionKind::ZeroOrOne  => RepetitionFlag::ZeroOrOne,
                    RepetitionKind::ZeroOrMore => RepetitionFlag::ZeroOrMore,
                    RepetitionKind::OneOrMore  => RepetitionFlag::OneOrMore,
                    RepetitionKind::Range(_) => panic!("The '{n,m}' repetition in #[regex] is currently unsupported."),
                };

                let mut node = Node::from_hir(repetition.hir.into_kind())?;

                node.make_repeat(flag);

                Some(node)
            },
            HirKind::Group(group) => {
                let mut fork = Fork::default();

                fork.insert(Node::from_hir(group.hir.into_kind())?);

                Some(Node::from(fork))
            },
            // This handles classes with non-ASCII Unicode ranges
            HirKind::Class(ref class) if !class_is_ascii(class) => {
                match class {
                    Class::Unicode(unicode) => {
                        let mut branches =
                            unicode.iter()
                                .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
                                .map(|seq| Branch::new(seq.into()));

                        branches.next().map(|branch| {
                            let mut node = Node::Branch(branch);

                            for branch in branches {
                                node.insert(Node::Branch(branch));
                            }

                            node
                        })
                    },
                    Class::Bytes(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                }
            },
            _ => {
                let mut regex = Regex::default();

                Regex::from_hir_internal(&hir, &mut regex);

                if regex.len() == 0 {
                    None
                } else {
                    Some(Branch::new(regex).into())
                }
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
                Pattern::Byte(byte)
            }).collect(),
            offset: 0,
        }
    }

    fn from_hir_internal(hir: &HirKind, regex: &mut Regex) -> bool {
        match hir {
            HirKind::Empty => true,
            HirKind::Literal(literal) => {
                use self::hir::Literal;

                match literal {
                    Literal::Unicode(unicode) => {
                        let mut buf = [0u8; 4];

                        regex.patterns.extend(
                            unicode
                                .encode_utf8(&mut buf)
                                .bytes()
                                .map(Pattern::Byte)
                        );
                    },
                    Literal::Byte(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                };

                true
            },
            HirKind::Class(class) => {
                if !class_is_ascii(&class) {
                    return false;
                }

                match class {
                    Class::Unicode(unicode) => {
                        let mut class = unicode
                            .iter()
                            .map(|range| {
                                let (start, mut end) = (range.start(), range.end());

                                static NON_ASCII: &str = "Non-ASCII ranges in #[regex] classes are currently unsupported.";

                                match end as u32 {
                                    0x10FFFF => end = 0xFF as char,
                                    _        => assert!(end.is_ascii(), NON_ASCII),
                                }

                                assert!(start.is_ascii(), NON_ASCII);

                                if start == end {
                                    Pattern::Byte(start as u8)
                                } else {
                                    Pattern::Range(start as u8, end as u8)
                                }
                            })
                            .collect::<Vec<_>>();

                        match class.len() {
                            0 => {},
                            1 => regex.patterns.push(class.remove(0)),
                            _ => regex.patterns.push(Pattern::Class(class)),
                        }
                    },
                    Class::Bytes(_) => panic!("Invalid Unicode codepoint in #[regex]."),
                }

                true
            },
            HirKind::WordBoundary(_) => panic!("Word boundaries in #[regex] are currently unsupported."),
            HirKind::Anchor(_) => panic!("Anchors in #[regex] are currently unsupported."),
            _ => false
        }
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.patterns[self.offset..]
    }

    pub fn first(&self) -> &Pattern {
        self.patterns().get(0).unwrap()
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

    pub fn common_prefix(&self, other: &Regex) -> Option<Pattern> {
        self.first().intersect(other.first())
    }

    pub fn unshift(&mut self) -> Option<&Pattern> {
        if self.len() == 0 {
            return None;
        }

        let offset = self.offset;

        self.offset += 1;

        self.patterns.get(offset)
    }

    pub fn consume(&mut self) -> &[Pattern] {
        let offset = self.offset;
        self.offset = self.patterns.len();

        &self.patterns[offset..]
    }
}

impl From<Pattern> for Regex {
    fn from(pat: Pattern) -> Regex {
        let mut regex = Regex::default();

        regex.patterns.push(pat);

        regex
    }
}

impl From<Utf8Sequence> for Regex {
    fn from(seq: Utf8Sequence) -> Self {
        let patterns =
            seq.as_slice()
               .iter()
               .cloned()
               .map(Pattern::from)
               .collect::<Vec<_>>();

        Regex {
            patterns,
            offset: 0,
        }
    }
}

fn class_is_ascii(class: &Class) -> bool {
    match class {
        Class::Unicode(unicode) => {
            unicode.iter().all(|range| {
                let start = range.start() as u32;
                let end = range.end() as u32;

                start < 128 && (end < 128 || end == 0x10FFFF)
            })
        },
        Class::Bytes(_) => false,
    }
}

impl From<Utf8Range> for Pattern {
    fn from(range: Utf8Range) -> Pattern {
        if range.start == range.end {
            Pattern::Byte(range.start)
        } else {
            Pattern::Range(range.start, range.end)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepetitionFlag {
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
    if byte >= 0x20 && byte <= 127 {
        write!(f, "{}", byte as char)
    } else {
        write!(f, "{:02X?}", byte)
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

impl fmt::Debug for RepetitionFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepetitionFlag::ZeroOrMore => f.write_str("*"),
            RepetitionFlag::OneOrMore => f.write_str("+"),
            RepetitionFlag::ZeroOrOne => f.write_str("?"),
        }
    }
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("'")?;

        for pattern in self.patterns() {
            write!(f, "{:?}", pattern)?;
        }

        f.write_str("'")
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

    pub fn contains(&self, other: &Pattern) -> bool {
        match other {
            Pattern::Byte(x) => self.contains_range(*x, *x),
            Pattern::Range(a, b) => self.contains_range(*a, *b),
            Pattern::Class(class) => {
                class.iter().all(|pat| self.contains(pat))
            },
        }
    }

    fn contains_range(&self, xa: u8, xb: u8) -> bool {
        match self {
            Pattern::Byte(a) => *a == xa && *a == xb,
            Pattern::Range(a, b) => {
                (*a <= xa && xa <= *b) && (*a <= xb && xb <= *b)
            },
            Pattern::Class(class) => {
                class.iter().any(|pat| pat.contains_range(xa, xb))
            },
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

            // Shorter first
            _ => match self.len().partial_cmp(&other.len()) {
                // Equal length != equal patterns, so let's not do that!
                Some(Ordering::Equal) => None,
                ordering              => ordering
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::Ident;

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

    fn mock_leaf() -> Leaf<'static> {
        use proc_macro2::Span;

        static mut MOCK_VARIANT: Option<&'static Ident> = None;

        let variant = unsafe {
            if MOCK_VARIANT.is_none() {
                let variant = Ident::new("mock", Span::call_site());
                let variant = Box::into_raw(Box::new(variant));

                MOCK_VARIANT = Some(&*variant);
            }

            MOCK_VARIANT.unwrap()
        };

        Leaf {
            token: variant,
            callback: None,
        }
    }

    fn branch(node: Node) -> Option<Branch> {
        match node {
            Node::Branch(branch) => Some(branch),
            Node::Fork(fork) => Some(fork.arms[0].clone()),
            _ => None
        }
    }

    #[test]
    fn branch_regex_number() {
        let leaf = mock_leaf();
        let regex = "[1-9][0-9]*";
        let b = branch(Node::from_regex(regex, leaf)).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'1', b'9')]);

        let b = branch(*b.then.unwrap()).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'0', b'9')]);
    }

    #[test]
    fn regex_ident() {
        let leaf = mock_leaf();
        let regex = "[a-zA-Z_$][a-zA-Z0-9_$]*";
        let b = branch(Node::from_regex(regex, leaf)).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                    Pattern::Byte(b'$'),
                    Pattern::Range(b'A', b'Z'),
                    Pattern::Byte(b'_'),
                    Pattern::Range(b'a', b'z'),
            ])
        ]);

        let b = branch(*b.then.unwrap()).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                Pattern::Byte(b'$'),
                Pattern::Range(b'0', b'9'),
                Pattern::Range(b'A', b'Z'),
                Pattern::Byte(b'_'),
                Pattern::Range(b'a', b'z'),
            ])
        ]);
    }

    #[test]
    fn regex_hex() {
        let leaf = mock_leaf();
        let regex = "0x[0-9a-fA-F]+";
        let b = branch(Node::from_regex(regex, leaf)).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Byte(b'0'),
            Pattern::Byte(b'x'),
        ]);

        let b = branch(*b.then.unwrap()).unwrap();

        assert_eq!(b.regex.patterns(), &[
            Pattern::Class(vec![
                Pattern::Range(b'0', b'9'),
                Pattern::Range(b'A', b'F'),
                Pattern::Range(b'a', b'f'),
            ])
        ]);
    }

    #[test]
    fn regex_unshift() {
        let leaf = mock_leaf();
        let regex = "abc";
        let mut r = branch(Node::from_regex(regex, leaf)).unwrap().regex;

        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'a'),
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Some(&Pattern::Byte(b'a')));
        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Some(&Pattern::Byte(b'b')));
        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Some(&Pattern::Byte(b'c')));
        assert_eq!(r.patterns(), &[]);
        assert_eq!(r.unshift(), None);
    }
}
