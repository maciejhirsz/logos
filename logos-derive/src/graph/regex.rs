use std::fmt;

use regex_syntax::hir::{self, Class, ClassUnicode, Hir, HirKind, Literal, RepetitionKind};
use regex_syntax::{ParserBuilder, Parser, Error as RError};
use proc_macro2::Span;
use beef::lean::Cow;

use crate::graph::{Graph, NodeBody, NodeId, Range, Rope, Fork};
use crate::Error;

impl<Leaf> Graph<Leaf> {
    pub fn regex(&mut self, utf8: bool, source: &str, span: Span, then: NodeId) -> Result<NodeBody<Leaf>, Error> {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let hir = match builder.build().parse(source) {
            Ok(hir) => hir.into_kind(),
            Err(err) => return spanned_error(err, span),
        };

        // By serendipity we don't have to reserve an id for the root
        // node of the regex. Patterns like `[0-9]*` could potentially match
        // empty string, resulting in an infinite loop in parsing.
        let first_id = Err("#[regex]: Pattern can match an empty string. \
                            Try switching * into +".into());

        match self.parse_hir(hir, first_id, then, None) {
            Ok(node) => Ok(node),
            Err(err) => spanned_error(err, span),
        }
    }

    fn parse_hir(
        &mut self,
        hir: HirKind,
        id: Result<NodeId, ParseError>,
        then: NodeId,
        miss: Option<NodeId>,
    ) -> Result<NodeBody<Leaf>, ParseError> {
        match hir {
            HirKind::Empty => Ok(Fork::new().miss(miss).into()),
//             HirKind::Alternation(alternation) => {
//                 let mut fork = Fork::default();

//                 for hir in alternation.into_iter().map(Hir::into_kind) {
//                     if let Some(node) = Node::from_hir(hir) {
//                         fork.insert(node);
//                     } else {
//                         fork.kind = ForkKind::Maybe;
//                     }
//                 }

//                 Some(Node::from(fork))
//             }
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
            HirKind::Concat(concat) => {
                // We'll be writing from the back, so need to allocate enough
                // space here. Worst case scenario is all unicode codepoints
                // producing 4 byte utf8 sequences
                let mut ropebuf = vec![0; concat.len() * 4];
                let mut cur = ropebuf.len();
                let mut end = ropebuf.len();

                for hir in concat.into_iter().rev().map(Hir::into_kind) {
                    match hir {
                        HirKind::Literal(Literal::Unicode(unicode)) => {
                            cur -= unicode.len_utf8();
                            unicode.encode_utf8(&mut ropebuf[cur..]);
                        },
                        HirKind::Literal(Literal::Byte(byte)) => {
                            cur -= 1;
                            ropebuf[cur] = byte;
                        },
                        hir => {
                            end = cur;
                            Err(format!("#[regex] unsupported HIR: {:#?}", hir))?
                        },
                    }
                }

                Ok(Rope::new(&ropebuf[cur..end], then).miss(miss).into())
            },
            HirKind::Repetition(repetition) => {
                if matches!(id, Ok(id) if id == then) {
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
                        let id = id?;
                        self.parse_hir(hir, Ok(id), id, Some(then))
                    },
                    RepetitionKind::OneOrMore => {
                        // Parse the loop first
                        let nid = self.reserve();
                        let next = self.parse_hir(hir.clone(), Ok(nid.get()), nid.get(), Some(then))?;
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
            hir => Err(format!("Internal Error: Unimplemented Regex HIR:\n\n{:#?}", hir))?,
        }
    }
}

pub struct ParseError(Cow<'static, str>);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<RError> for ParseError {
    fn from(err: RError) -> ParseError {
        ParseError(err.to_string().into())
    }
}

impl From<&'static str> for ParseError {
    fn from(err: &'static str) -> ParseError {
        ParseError(err.into())
    }
}

impl From<String> for ParseError {
    fn from(err: String) -> ParseError {
        ParseError(err.into())
    }
}

fn is_ascii_or_bytes(class: &ClassUnicode) -> bool {
    class.iter().all(|range| {
        let start = range.start() as u32;
        let end = range.end() as u32;

        start < 128 && (end < 128 || end == 0x0010_FFFF)
    })
}

fn spanned_error<E, T>(err: E, span: Span) -> Result<T, Error>
where
    E: std::fmt::Display,
{
    Err(Error::new(format!("{}\n\nIn this declaration:", err), span))
}