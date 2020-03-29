use std::fmt::Debug;
use std::cmp::max;

use regex_syntax::hir::{Class, ClassUnicode, HirKind, Literal, RepetitionKind};
use regex_syntax::ParserBuilder;
use utf8_ranges::Utf8Sequences;

use crate::graph::{Graph, Disambiguate, NodeId, Range, Rope, Fork};
use crate::error::Result;

impl<Leaf: Disambiguate + Debug> Graph<Leaf> {
    pub fn regex(&mut self, utf8: bool, source: &str, then: NodeId) -> Result<(usize, NodeId)> {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let hir = builder.build().parse(source)?.into_kind();

        self.parse_hir(hir, then, None)
    }

    fn parse_hir(
        &mut self,
        hir: HirKind,
        then: NodeId,
        miss: Option<NodeId>,
    ) -> Result<(usize, NodeId)> {
        match hir {
            HirKind::Empty => {
                let id = match miss {
                    Some(miss) => self.merge(miss, then),
                    None => then,
                };
                Ok((0, id))
            },
            HirKind::Alternation(alternation) => {
                let mut fork = Fork::new().miss(miss);
                let mut longest = 0;

                for hir in alternation {
                    let (len, id) = self.parse_hir(hir.into_kind(), then, None)?;
                    let alt = self.fork_off(id);

                    longest = max(longest, len);

                    fork.merge(alt, self);
                }

                Ok((longest, self.push(fork)))
            }
            HirKind::Literal(literal) => {
                let pattern = match literal {
                    Literal::Unicode(unicode) => {
                        unicode.encode_utf8(&mut [0; 4]).as_bytes().to_vec()
                    },
                    Literal::Byte(byte) => {
                        [byte].to_vec()
                    },
                };
                Ok((pattern.len(), self.push(Rope::new(pattern, then).miss(miss))))
            },
            HirKind::Concat(mut concat) => {
                // We'll be writing from the back, so need to allocate enough
                // space here. Worst case scenario is all unicode codepoints
                // producing 4 byte utf8 sequences
                let mut ropebuf = vec![Range::from(0); concat.len() * 4];
                let mut cur = ropebuf.len();
                let mut end = ropebuf.len();
                let mut then = then;
                let mut total_len = 0;

                let mut handle_bytes = |graph: &mut Self, hir, then: &mut NodeId| {
                    match hir {
                        HirKind::Literal(Literal::Unicode(u)) => {
                            cur -= u.len_utf8();
                            for (i, byte) in u.encode_utf8(&mut [0; 4]).bytes().enumerate() {
                                ropebuf[cur + i] = byte.into();
                            }
                            None
                        },
                        HirKind::Literal(Literal::Byte(byte)) => {
                            cur -= 1;
                            ropebuf[cur] = byte.into();
                            None
                        },
                        HirKind::Class(Class::Unicode(class)) if is_one_ascii(&class) => {
                            cur -= 1;
                            ropebuf[cur] = class.ranges()[0].into();
                            None
                        },
                        HirKind::Class(Class::Bytes(class)) if class.ranges().len() == 1 => {
                            cur -= 1;
                            ropebuf[cur] = class.ranges()[0].into();
                            None
                        },
                        hir => {
                            let len = end - cur;
                            if len != 0 {
                                *then = graph.push(Rope::new(&ropebuf[cur..end], *then));
                                end = cur;
                            }
                            Some((len, hir))
                        },
                    }
                };

                for hir in concat.drain(1..).rev() {
                    if let Some((len, hir)) = handle_bytes(self, hir.into_kind(), &mut then) {
                        let (nlen, next) = self.parse_hir(hir, then, None)?;

                        total_len += len + nlen;

                        then = next
                    }
                }

                match handle_bytes(self, concat.remove(0).into_kind(), &mut then) {
                    None => {
                        total_len += end - cur;

                        Ok((total_len, self.push(Rope::new(&ropebuf[cur..end], then).miss(miss))))
                    },
                    Some((len, hir)) => {
                        let (nlen, id) = self.parse_hir(hir, then, miss)?;

                        total_len += len + nlen;

                        Ok((total_len, id))
                    },
                }
            },
            HirKind::Repetition(repetition) => {
                if !repetition.greedy {
                    Err("#[regex]: non-greedy parsing is currently unsupported.")?;
                }

                let hir = repetition.hir.into_kind();
                match repetition.kind {
                    RepetitionKind::ZeroOrOne => {
                        let miss = match miss {
                            Some(id) => self.merge(id, then),
                            None => then,
                        };

                        let (_, id) = self.parse_hir(hir, then, Some(miss))?;

                        Ok((0, id))
                    },
                    RepetitionKind::ZeroOrMore => {
                        let miss = match miss {
                            Some(id) => self.merge(id, then),
                            None => then,
                        };

                        let this = self.reserve();
                        let (_, id) = self.parse_hir(hir, this.get(), Some(miss))?;
                        // Move a fork to the reserved id
                        let fork = self.fork_off(id);
                        let id = self.insert(this, fork);

                        Ok((0, id))
                    },
                    RepetitionKind::OneOrMore => {
                        // Parse the loop first
                        let nid = self.reserve();
                        let (_, next) = self.parse_hir(hir.clone(), nid.get(), None)?;
                        let next = self.merge(next, then);
                        let next = self.fork_off(next);
                        let next = self.insert(nid, next);

                        // Then parse the same tree into first node, attaching loop
                        self.parse_hir(hir, next, miss)
                    },
                    RepetitionKind::Range(..) => {
                        Err("#[regex]: {n,m} repetition range is currently unsupported.")?
                    },
                }
            },
            HirKind::Group(group) => {
                let hir = group.hir.into_kind();

                self.parse_hir(hir, then, miss)
            },
            HirKind::Class(Class::Unicode(class)) if !is_ascii(&class) => {
                let mut ropes = class
                    .iter()
                    .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
                    .map(|sequence| Rope::new(sequence.as_slice(), then))
                    .collect::<Vec<_>>();

                if ropes.len() == 0 {
                    let rope = ropes.remove(0);

                    return Ok((rope.pattern.len(), self.push(rope.miss(miss))));
                }

                let mut root = Fork::new().miss(miss);
                let mut longest = 0;

                for rope in ropes {
                    longest = max(longest, rope.pattern.len());

                    let fork = rope.into_fork(self);
                    root.merge(fork, self);
                }

                Ok((longest, self.push(root)))
            },
            HirKind::Class(class) => {
                let mut fork = Fork::new().miss(miss);
                let class: Vec<Range> = match class {
                    Class::Unicode(u) => {
                        u.iter().copied().map(Into::into).collect()
                    }
                    Class::Bytes(b) => {
                        b.iter().copied().map(Into::into).collect()
                    }
                };

                for range in class {
                    fork.add_branch(range, then, self);
                }

                Ok((1, self.push(fork)))
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

        assert_eq!(len, 6);
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

        assert_eq!(len, 1);
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
    fn regex_combine_len() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let (len, _) = graph.regex(true, "(fooz|bar)+qux", leaf).unwrap();

        assert_eq!(len, 7); // foozqux
    }
}