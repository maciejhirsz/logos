use regex_syntax::hir::{Class, ClassUnicode, HirKind, Literal, RepetitionKind};
use regex_syntax::ParserBuilder;

use crate::graph::{Graph, NodeBody, NodeId, Range, Rope, Fork};
use crate::error::Result;

impl<Leaf: std::fmt::Debug> Graph<Leaf> {
    pub fn regex(&mut self, utf8: bool, source: &str, then: NodeId) -> Result<NodeId> {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let hir = builder.build().parse(source)?.into_kind();

        let id = self.reserve();
        let node = self.parse_hir(hir, id.get(), then, None)?;

        Ok(self.insert(id, node))
    }

    fn parse_hir(
        &mut self,
        hir: HirKind,
        id: NodeId,
        then: NodeId,
        miss: Option<NodeId>,
    ) -> Result<NodeBody<Leaf>> {
        match hir {
            HirKind::Empty => {
                let miss = match miss {
                    Some(miss) => self.merge(miss, then),
                    None => then,
                };
                Ok(Fork::new().miss(miss).into())
            },
            HirKind::Alternation(alternation) => {
                let mut fork = Fork::new().miss(miss);

                for hir in alternation {
                    let alt = match self.parse_hir(hir.into_kind(), id, then, None)? {
                        NodeBody::Fork(fork) => fork,
                        NodeBody::Rope(rope) => rope.fork_off(self),
                        NodeBody::Leaf(_) => {
                            Err("Internal Error: Regex produced a leaf node.")?
                        }
                    };

                    fork.merge(alt, self);
                }

                Ok(fork.into())
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

                Ok(Rope::new(pattern, then).miss(miss).into())
            },
            HirKind::Concat(mut concat) => {
                // We'll be writing from the back, so need to allocate enough
                // space here. Worst case scenario is all unicode codepoints
                // producing 4 byte utf8 sequences
                let mut ropebuf = vec![0; concat.len() * 4];
                let mut cur = ropebuf.len();
                let mut end = ropebuf.len();
                let mut then = then;

                let mut handle_bytes = |graph: &mut Self, hir, then: &mut NodeId| {
                    match hir {
                        HirKind::Literal(Literal::Unicode(unicode)) => {
                            cur -= unicode.len_utf8();
                            unicode.encode_utf8(&mut ropebuf[cur..]);
                            None
                        },
                        HirKind::Literal(Literal::Byte(byte)) => {
                            cur -= 1;
                            ropebuf[cur] = byte;
                            None
                        },
                        hir => {
                            if end != cur {
                                *then = graph.push(Rope::new(&ropebuf[cur..end], *then));
                                end = cur;
                            }
                            Some(hir)
                        },
                    }
                };

                for hir in concat.drain(1..).rev() {
                    if let Some(hir) = handle_bytes(self, hir.into_kind(), &mut then) {
                        let nid = self.reserve();
                        let next = self.parse_hir(hir, nid.get(), then, None)?;

                        then = self.insert(nid, next);
                    }
                }

                match handle_bytes(self, concat.remove(0).into_kind(), &mut then) {
                    None => {
                        Ok(Rope::new(&ropebuf[cur..end], then).miss(miss).into())
                    },
                    Some(hir) => {
                        self.parse_hir(hir, id, then, miss)
                    },
                }
            },
            HirKind::Repetition(repetition) => {
                if id == then {
                    Err("#[regex]: Repetition inside a repetition.")?;
                }
                if !repetition.greedy {
                    Err("#[regex]: Non-greedy parsing is currently unsupported.")?;
                }

                let hir = repetition.hir.into_kind();

                match repetition.kind {
                    RepetitionKind::ZeroOrOne => {
                        self.parse_hir(hir, id, then, Some(then))
                    },
                    RepetitionKind::ZeroOrMore => {
                        self.parse_hir(hir, id, id, Some(then))
                    },
                    RepetitionKind::OneOrMore => {
                        // Parse the loop first
                        let nid = self.reserve();
                        let next = self.parse_hir(hir.clone(), nid.get(), nid.get(), Some(then))?;
                        let next = self.insert(nid, next);

                        // Then parse the same tree into first node, attaching loop
                        self.parse_hir(hir, id, next, miss)
                    },
                    RepetitionKind::Range(..) => {
                        Err("#[regex]: {n,m} repetition range is currently unsupported.")?
                    },
                }
            },
            HirKind::Group(group) => {
                let hir = group.hir.into_kind();

                self.parse_hir(hir, id, then, miss)
            },
            HirKind::Class(Class::Unicode(class)) if !is_ascii_or_bytes(&class) => {
                Err("No support for unicode just yet!")?
                // let mut branches = unicode
                //     .iter()
                //     .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
                //     .map(Branch::new);

                // branches.next().map(|branch| {
                //     let mut node = Node::Branch(branch);

                //     for branch in branches {
                //         node.insert(Node::Branch(branch));
                //     }

                //     node
                // })
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
                    fork.add_branch(range, then);
                }

                Ok(fork.into())
            },
            HirKind::WordBoundary(_) => {
                Err("#[regex]: Word boundaries are currently unsupported.")?
            },
            HirKind::Anchor(_) => {
                Err("#[regex]: Anchors in #[regex] are currently unsupported.")?
            },
        }
    }
}

fn is_ascii_or_bytes(class: &ClassUnicode) -> bool {
    class.iter().all(|range| {
        let start = range.start() as u32;
        let end = range.end() as u32;

        start < 128 && (end < 128 || end == 0x0010_FFFF)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rope() {
        let mut graph = Graph::new();

        let leaf = graph.push(NodeBody::Leaf("LEAF"));
        let parsed = graph.regex(true, "foobar", leaf).unwrap();

        assert_eq!(
            graph[parsed].body,
            NodeBody::Rope(Rope::new("foobar", leaf)),
        )
    }

    #[test]
    fn alternation() {
        let mut graph = Graph::new();

        let leaf = graph.push(NodeBody::Leaf("LEAF"));
        let parsed = graph.regex(true, "a|b", leaf).unwrap();

        assert_eq!(
            graph[parsed].body,
            NodeBody::Fork(
                Fork::new()
                    .branch(b'a', leaf)
                    .branch(b'b', leaf)
            ),
        );
    }

    #[test]
    fn repeat() {
        let mut graph = Graph::new();

        let leaf = graph.push(NodeBody::Leaf("LEAF"));
        let parsed = graph.regex(true, "[a-z]*", leaf).unwrap();

        assert_eq!(
            graph[parsed].body,
            NodeBody::Fork(
                Fork::new()
                    .branch('a'..='z', parsed) // goto self == loop
                    .miss(leaf)
            ),
        );
    }

    #[test]
    fn maybe() {
        let mut graph = Graph::new();

        let leaf = graph.push(NodeBody::Leaf("LEAF"));
        let parsed = graph.regex(true, "[a-z]?", leaf).unwrap();

        assert_eq!(
            graph[parsed].body,
            NodeBody::Fork(
                Fork::new()
                    .branch('a'..='z', leaf)
                    .miss(leaf)
            ),
        );
    }
}