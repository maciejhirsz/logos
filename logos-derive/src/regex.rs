use utf8_ranges::{Utf8Sequences, Utf8Sequence, Utf8Range};
use regex_syntax::hir::{self, Hir, HirKind, Class};
use regex_syntax::ParserBuilder;
use std::fmt;

use crate::tree::{Node, Fork, ForkKind, Branch, Leaf};

mod pattern;

pub use self::pattern::Pattern;

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Regex {
    patterns: Vec<Pattern>,
}

impl<'a> Node<'a> {
    pub fn from_sequence<Source>(source: Source, leaf: Leaf<'a>) -> Self
    where
        Source: AsRef<[u8]>,
    {
        let regex = Regex::sequence(source);

        if regex.len() == 0 {
            panic!("Empty #[token] string in variant: {}!", leaf.token);
        }

        Node::new(regex, leaf)
    }

    pub fn from_regex(source: &str, utf8: bool, leaf: Option<Leaf<'a>>) -> Self {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let hir = match builder.build().parse(source) {
            Ok(hir) => hir.into_kind(),
            Err(err) => {
                if let Some(leaf) = leaf {
                    panic!("Unable to parse #[regex] for variant: {}!\n\n{:#?}", leaf.token, err);
                } else {
                    panic!("Unable to parse #[regex]! {:#?}", err);
                }
            },
        };

        let mut node = Self::from_hir(hir).expect("Unable to produce a valid tree for #[regex]");

        if let Some(leaf) = leaf {
            let leaf = Node::Leaf(leaf);

            node.chain(&leaf);
        }

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
            HirKind::Class(ref class) if !is_ascii_or_bytes(class) => {
                match class {
                    Class::Unicode(unicode) => {
                        let mut branches =
                            unicode.iter()
                                .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
                                .map(|seq| Branch::new(seq));

                        branches.next().map(|branch| {
                            let mut node = Node::Branch(branch);

                            for branch in branches {
                                node.insert(Node::Branch(branch));
                            }

                            node
                        })
                    },
                    Class::Bytes(_) => {
                        // `is_ascii_or_bytes` check shouldn't permit us to branch here

                        panic!("Internal Error")
                    },
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

    pub fn sequence<Source>(source: Source) -> Self
    where
        Source: AsRef<[u8]>,
    {
        Regex {
            patterns: source.as_ref().iter().cloned().map(Pattern::Byte).collect(),
        }
    }

    fn from_hir_internal(hir: &HirKind, regex: &mut Regex) -> bool {
        match hir {
            HirKind::Empty => true,
            HirKind::Literal(literal) => {
                use self::hir::Literal;

                match literal {
                    Literal::Unicode(unicode) => {
                        regex.patterns.extend(
                            unicode
                                .encode_utf8(&mut [0; 4])
                                .bytes()
                                .map(Pattern::Byte)
                        );
                    },
                    Literal::Byte(byte) => {
                        regex.patterns.push(Pattern::Byte(*byte));
                    },
                };

                true
            },
            HirKind::Class(class) => {
                if !is_ascii_or_bytes(&class) {
                    return false;
                }

                let mut class: Vec<_> = match class {
                    Class::Unicode(unicode) => {
                        unicode
                            .iter()
                            .map(|range| {
                                let (start, end) = (range.start(), range.end());

                                let start = start as u8;
                                let end = if end == '\u{10FFFF}' { 0xFF } else { end as u8 };

                                if start == end {
                                    Pattern::Byte(start)
                                } else {
                                    Pattern::Range(start, end)
                                }
                            })
                            .collect()
                    },
                    Class::Bytes(bytes) => {
                        bytes
                            .iter()
                            .map(|range| {
                                let (start, end) = (range.start(), range.end());

                                if start == end {
                                    Pattern::Byte(start)
                                } else {
                                    Pattern::Range(start, end)
                                }
                            })
                            .collect()
                    },
                };

                match class.len() {
                    0 => {},
                    1 => regex.patterns.push(class.remove(0)),
                    _ => regex.patterns.push(Pattern::Class(class)),
                }

                true
            },
            HirKind::WordBoundary(_) => panic!("Word boundaries in #[regex] are currently unsupported."),
            HirKind::Anchor(_) => panic!("Anchors in #[regex] are currently unsupported."),
            _ => false
        }
    }

    pub fn patterns(&self) -> &[Pattern] {
        &self.patterns
    }

    pub fn first(&self) -> &Pattern {
        self.patterns.first().expect("Internal Error: Empty Regex")
    }

    pub fn first_mut(&mut self) -> &mut Pattern {
        self.patterns.first_mut().expect("Internal Error: Empty Regex")
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
                self.patterns.drain(..len).count();
                other.patterns.drain(..len).count();

                Some(Regex {
                    patterns,
                })
            }
        }
    }

    pub fn common_prefix(&self, other: &Regex) -> Option<Pattern> {
        self.first().intersect(other.first())
    }

    pub fn unshift(&mut self) -> Pattern {
        self.patterns.remove(0)
    }

    pub fn extend(&mut self, patterns: &[Pattern]) {
        self.patterns.extend(patterns.iter().cloned());
    }
}

impl From<Pattern> for Regex {
    fn from(pat: Pattern) -> Regex {
        Regex {
            patterns: vec![pat],
        }
    }
}

impl<'a> From<&'a [Pattern]> for Regex {
    fn from(patterns: &'a [Pattern]) -> Regex {
        Regex {
            patterns: patterns.to_vec(),
        }
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
        }
    }
}

impl<'a> From<&'a str> for Regex {
    fn from(seq: &'a str) -> Self {
        Regex::sequence(seq)
    }
}

fn is_ascii_or_bytes(class: &Class) -> bool {
    match class {
        Class::Unicode(unicode) => {
            unicode.iter().all(|range| {
                let start = range.start() as u32;
                let end = range.end() as u32;

                start < 128 && (end < 128 || end == 0x10FFFF)
            })
        },
        Class::Bytes(_) => true,
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

#[cfg(test)]
mod test {
    use super::*;

    fn branch(node: Node) -> Option<Branch> {
        match node {
            Node::Branch(branch) => Some(branch),
            Node::Fork(fork) => Some(fork.arms[0].clone()),
            _ => None
        }
    }

    #[test]
    fn branch_regex_number() {
        let regex = "[1-9][0-9]*";
        let b = branch(Node::from_regex(regex, true, None)).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'1', b'9')]);

        let b = branch(*b.then.unwrap()).unwrap();

        assert_eq!(b.regex.patterns(), &[Pattern::Range(b'0', b'9')]);
    }

    #[test]
    fn regex_ident() {
        let regex = "[a-zA-Z_$][a-zA-Z0-9_$]*";
        let b = branch(Node::from_regex(regex, true, None)).unwrap();

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
        let regex = "0x[0-9a-fA-F]+";
        let b = branch(Node::from_regex(regex, true, None)).unwrap();

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
        let regex = "abc";
        let mut r = branch(Node::from_regex(regex, true, None)).unwrap().regex;

        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'a'),
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Pattern::Byte(b'a'));
        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'b'),
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Pattern::Byte(b'b'));
        assert_eq!(r.patterns(), &[
            Pattern::Byte(b'c'),
        ]);

        assert_eq!(r.unshift(), Pattern::Byte(b'c'));
        assert_eq!(r.patterns(), &[]);
    }
}
