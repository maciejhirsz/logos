use std::fmt::Debug;
use std::cmp::min;
use std::convert::TryFrom;

use regex_syntax::hir::{Class, ClassUnicode, Hir, HirKind, Literal, RepetitionKind};
use regex_syntax::ParserBuilder;
use utf8_ranges::Utf8Sequences;

use crate::graph::{Graph, Disambiguate, Node, NodeId, Range, Rope, Fork};
use crate::error::{Error, Result};

/// Middle Intermediate Representation of the regex, built from
/// `regex_syntax`'s `Hir`. The goal here is to strip and canonicalize
/// the tree, so that we don't have to do transformations later on the
/// graph, with the potential of running into looping references.
#[derive(Clone, Debug)]
enum Mir {
    Empty,
    Loop(Box<Mir>),
    Maybe(Box<Mir>),
    Concat(Vec<Mir>),
    Alternation(Vec<Mir>),
    Class(Class),
    Literal(Literal),
}

impl TryFrom<Hir> for Mir {
    type Error = Error;

    fn try_from(hir: Hir) -> Result<Mir> {
        match hir.into_kind() {
            HirKind::Empty => {
                Ok(Mir::Empty)
            },
            HirKind::Concat(concat) => {
                let mut out = Vec::with_capacity(concat.len());

                for hir in concat {
                    match Mir::try_from(hir)? {
                        Mir::Concat(nested) => out.extend(nested),
                        mir => out.push(mir),
                    }
                }

                Ok(Mir::Concat(out))
            },
            HirKind::Alternation(alternation) => {
                let alternation = alternation
                    .into_iter()
                    .map(Mir::try_from)
                    .collect::<Result<_>>()?;

                Ok(Mir::Alternation(alternation))
            },
            HirKind::Literal(literal) => {
                Ok(Mir::Literal(literal))
            },
            HirKind::Class(class) => {
                Ok(Mir::Class(class))
            },
            HirKind::Repetition(repetition) => {
                if !repetition.greedy {
                    Err("#[regex]: non-greedy parsing is currently unsupported.")?;
                }

                let kind = repetition.kind;
                let mir = Mir::try_from(*repetition.hir)?;

                match kind {
                    RepetitionKind::ZeroOrOne => {
                        Ok(Mir::Maybe(Box::new(mir)))
                    },
                    RepetitionKind::ZeroOrMore => {
                        Ok(Mir::Loop(Box::new(mir)))
                    },
                    RepetitionKind::OneOrMore => {
                        Ok(Mir::Concat(vec![
                            mir.clone(),
                            Mir::Loop(Box::new(mir)),
                        ]))
                    },
                    RepetitionKind::Range(..) => {
                        Err("#[regex]: {n,m} repetition range is currently unsupported.")?
                    },
                }
            },
            HirKind::Group(group) => {
                Mir::try_from(*group.hir)
            },
            HirKind::WordBoundary(_) => {
                Err("#[regex]: word boundaries are currently unsupported.")?
            },
            HirKind::Anchor(_) => {
                Err("#[regex]: anchors in #[regex] are currently unsupported.")?
            },
        }
    }
}

impl<Leaf: Disambiguate + Debug> Graph<Leaf> {
    pub fn regex(&mut self, utf8: bool, source: &str, then: NodeId) -> Result<(usize, NodeId)> {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let hir = builder.build().parse(source)?;
        let mir = Mir::try_from(hir.clone())?;

        Ok(self.parse_mir(mir, then, None))
    }

    fn parse_mir(&mut self, mir: Mir, then: NodeId, miss: Option<NodeId>) -> (usize, NodeId) {
        match mir {
            Mir::Empty => (0, then),
            Mir::Loop(mir) => {
                let miss = match miss {
                    Some(id) => self.merge(id, then),
                    None => then,
                };
                let this = self.reserve();
                let (_, id) = self.parse_mir(*mir, this.get(), Some(miss));

                // Move the node to the reserved id
                let node = match &self[id] {
                    Node::Fork(fork) => Node::Fork(fork.clone()),
                    Node::Rope(rope) => Node::Rope(rope.clone()),
                    Node::Leaf(_) => unreachable!(),
                };
                let id = self.insert(this, node);

                (0, id)
            }
            Mir::Maybe(mir) => {
                let miss = match miss {
                    Some(id) => self.merge(id, then),
                    None => then,
                };
                let (_, id) = self.parse_mir(*mir, then, Some(miss));

                (0, id)
            },
            Mir::Alternation(alternation) => {
                let mut fork = Fork::new().miss(miss);
                let mut shortest = if alternation.len() > 0 { usize::max_value() } else { 0 };

                for mir in alternation {
                    let (len, id) = self.parse_mir(mir, then, None);
                    let alt = self.fork_off(id);

                    shortest = min(shortest, len);

                    fork.merge(alt, self);
                }

                (shortest, self.push(fork))
            }
            Mir::Literal(literal) => {
                let pattern = match literal {
                    Literal::Unicode(unicode) => {
                        unicode.encode_utf8(&mut [0; 4]).as_bytes().to_vec()
                    },
                    Literal::Byte(byte) => {
                        [byte].to_vec()
                    },
                };
                (pattern.len() * 2, self.push(Rope::new(pattern, then).miss(miss)))
            },
            Mir::Concat(mut concat) => {
                // We'll be writing from the back, so need to allocate enough
                // space here. Worst case scenario is all unicode codepoints
                // producing 4 byte utf8 sequences
                let mut ropebuf = vec![Range::from(0); concat.len() * 4];
                let mut cur = ropebuf.len();
                let mut end = ropebuf.len();
                let mut then = then;
                let mut total_len = 0;

                let mut handle_bytes = |graph: &mut Self, mir, then: &mut NodeId| {
                    match mir {
                        Mir::Literal(Literal::Unicode(u)) => {
                            cur -= u.len_utf8();
                            for (i, byte) in u.encode_utf8(&mut [0; 4]).bytes().enumerate() {
                                ropebuf[cur + i] = byte.into();
                            }
                            None
                        },
                        Mir::Literal(Literal::Byte(byte)) => {
                            cur -= 1;
                            ropebuf[cur] = byte.into();
                            None
                        },
                        Mir::Class(Class::Unicode(class)) if is_one_ascii(&class) => {
                            cur -= 1;
                            ropebuf[cur] = class.ranges()[0].into();
                            None
                        },
                        Mir::Class(Class::Bytes(class)) if class.ranges().len() == 1 => {
                            cur -= 1;
                            ropebuf[cur] = class.ranges()[0].into();
                            None
                        },
                        mir => {
                            if end > cur {
                                let rope = Rope::new(&ropebuf[cur..end], *then);
                                let len = rope.priority();

                                *then = graph.push(rope);
                                end = cur;

                                Some((len, mir))
                            } else {
                                Some((0, mir))
                            }
                        },
                    }
                };

                for mir in concat.drain(1..).rev() {
                    if let Some((len, mir)) = handle_bytes(self, mir, &mut then) {
                        let (nlen, next) = self.parse_mir(mir, then, None);

                        total_len += len + nlen;

                        then = next
                    }
                }

                match handle_bytes(self, concat.remove(0), &mut then) {
                    None => {
                        let rope = Rope::new(&ropebuf[cur..end], then).miss(miss);

                        total_len += rope.priority();

                        (total_len, self.push(rope))
                    },
                    Some((len, mir)) => {
                        let (nlen, id) = self.parse_mir(mir, then, miss);

                        total_len += len + nlen;

                        (total_len, id)
                    },
                }
            },
            Mir::Class(Class::Unicode(class)) if !is_ascii(&class) => {
                let mut ropes = class
                    .iter()
                    .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
                    .map(|sequence| Rope::new(sequence.as_slice(), then))
                    .collect::<Vec<_>>();

                if ropes.len() == 0 {
                    let rope = ropes.remove(0);

                    return (rope.pattern.len(), self.push(rope.miss(miss)));
                }

                let mut root = Fork::new().miss(miss);
                let mut shortest = usize::max_value();

                for rope in ropes {
                    shortest = min(shortest, rope.priority());

                    let fork = rope.into_fork(self);
                    root.merge(fork, self);
                }

                (shortest, self.push(root))
            },
            Mir::Class(class) => {
                let mut fork = Fork::new().miss(miss);
                let mut len = 2;

                let class: Vec<Range> = match class {
                    Class::Unicode(u) => {
                        u.iter().copied().map(Into::into).collect()
                    }
                    Class::Bytes(b) => {
                        b.iter().copied().map(Into::into).collect()
                    }
                };

                for range in class {
                    if range.as_byte().is_none() {
                        len = 1;
                    }
                    fork.add_branch(range, then, self);
                }

                (len, self.push(fork))
            },
        }
    }
}

fn is_ascii(class: &ClassUnicode) -> bool {
    class.iter().all(|range| {
        let start = range.start() as u32;
        let end = range.end() as u32;

        start < 128 && (end < 128 || end == 0x0010_FFFF)
    })
}

fn is_one_ascii(class: &ClassUnicode) -> bool {
    if class.ranges().len() != 1 {
        return false;
    }

    let range = &class.ranges()[0];
    let start = range.start() as u32;
    let end = range.end() as u32;

    start < 128 && (end < 128 || end == 0x0010_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Node;
    use pretty_assertions::assert_eq;

    #[test]
    fn rope() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let (len, parsed) = graph.regex(true, "foobar", leaf).unwrap();

        assert_eq!(len, 12);
        assert_eq!(
            graph[parsed],
            Node::Rope(Rope::new("foobar", leaf)),
        )
    }

    #[test]
    fn alternation() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let (len, parsed) = graph.regex(true, "a|b", leaf).unwrap();

        assert_eq!(len, 2);
        assert_eq!(
            graph[parsed],
            Node::Fork(
                Fork::new()
                    .branch(b'a', leaf)
                    .branch(b'b', leaf)
            ),
        );
    }

    #[test]
    fn repeat() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let (len, parsed) = graph.regex(true, "[a-z]*", leaf).unwrap();

        assert_eq!(len, 0);
        assert_eq!(
            graph[parsed],
            Node::Fork(
                Fork::new()
                    .branch('a'..='z', parsed) // goto self == loop
                    .miss(leaf)
            ),
        );
    }

    #[test]
    fn maybe() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let (len, parsed) = graph.regex(true, "[a-z]?", leaf).unwrap();

        assert_eq!(len, 0);
        assert_eq!(
            graph[parsed],
            Node::Fork(
                Fork::new()
                    .branch('a'..='z', leaf)
                    .miss(leaf)
            ),
        );
    }

    #[test]
    fn priorities() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));

        let regexes = [
            ("[a-z]+", 1),
            ("a|b", 2),
            ("a|[b-z]", 1),
            ("(foo)+", 6),
            ("foobar", 12),
            ("(fooz|bar)+qux", 12),
        ];

        for (regex, expected) in regexes.iter() {
            let (len, _) = graph.regex(true, regex, leaf).unwrap();
            assert_eq!(len, *expected);
        }
    }
}