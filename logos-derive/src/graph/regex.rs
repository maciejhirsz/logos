use regex_syntax::hir::{self, Class, Hir, HirKind};
use regex_syntax::{ParserBuilder, Parser, Error as RError};
use proc_macro2::Span;

use crate::graph::{Graph, NodeId, Fork};
use crate::Error;

impl<Leaf> Graph<Leaf> {
    pub fn regex(&mut self, utf8: bool, source: &str, span: Span, then: NodeId) -> Result<Fork, Error> {
        let mut builder = ParserBuilder::new();

        if !utf8 {
            builder.allow_invalid_utf8(true).unicode(false);
        }

        let fork = match self.regex_internal(builder.build(), source) {
            Ok(fork) => fork,
            Err(err) => return Err(Error::new(format!("{}\n\nIn this declaration:", err), span)),
        };

        match fork {
            Some(fork) => Ok(fork),
            None => Err(Error::new("Empty #[regex]", span)),
        }
    }

    fn regex_internal(&mut self, mut parser: Parser, source: &str) -> Result<Option<Fork>, RError> {
        let hir = parser.parse(source)?;

        // panic!("{:#?}", hir);

        self.from_hir(hir.into_kind())
    }

    fn from_hir(&mut self, hir: HirKind) -> Result<Option<Fork>, RError> {
        match hir {
            HirKind::Empty => Ok(None),
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
//             HirKind::Concat(concat) => {
//                 let mut concat = concat.into_iter().map(Hir::into_kind).collect::<Vec<_>>();
//                 let mut nodes = vec![];
//                 let mut read = 0;

//                 while concat.len() != read {
//                     let mut regex = Regex::default();

//                     let count = concat[read..]
//                         .iter()
//                         .take_while(|hir| Regex::from_hir_internal(hir, &mut regex))
//                         .count();

//                     if count != 0 {
//                         nodes.push(Branch::new(regex).into());
//                         read += count;
//                     } else if let Some(node) = Node::from_hir(concat.remove(read)) {
//                         nodes.push(node);
//                     }
//                 }

//                 let mut node = nodes.pop()?;

//                 for mut n in nodes.into_iter().rev() {
//                     n.chain(&node);

//                     node = n;
//                 }

//                 Some(node)
//             }
//             HirKind::Repetition(repetition) => {
//                 use self::hir::RepetitionKind;

//                 // FIXME?
//                 if !repetition.greedy {
//                     panic!("Non-greedy parsing in #[regex] is currently unsupported.")
//                 }

//                 let flag = match repetition.kind {
//                     RepetitionKind::ZeroOrOne => RepetitionFlag::ZeroOrOne,
//                     RepetitionKind::ZeroOrMore => RepetitionFlag::ZeroOrMore,
//                     RepetitionKind::OneOrMore => RepetitionFlag::OneOrMore,
//                     RepetitionKind::Range(_) => {
//                         panic!("The '{n,m}' repetition in #[regex] is currently unsupported.")
//                     }
//                 };

//                 let mut node = Node::from_hir(repetition.hir.into_kind())?;

//                 node.make_repeat(flag);

//                 Some(node)
//             }
//             HirKind::Group(group) => {
//                 let mut fork = Fork::default();

//                 fork.insert(Node::from_hir(group.hir.into_kind())?);

//                 Some(Node::from(fork))
//             }
//             // This handles classes with non-ASCII Unicode ranges
//             HirKind::Class(ref class) if !is_ascii_or_bytes(class) => {
//                 match class {
//                     Class::Unicode(unicode) => {
//                         let mut branches = unicode
//                             .iter()
//                             .flat_map(|range| Utf8Sequences::new(range.start(), range.end()))
//                             .map(Branch::new);

//                         branches.next().map(|branch| {
//                             let mut node = Node::Branch(branch);

//                             for branch in branches {
//                                 node.insert(Node::Branch(branch));
//                             }

//                             node
//                         })
//                     }
//                     Class::Bytes(_) => {
//                         // `is_ascii_or_bytes` check shouldn't permit us to branch here

//                         panic!("Internal Error")
//                     }
//                 }
//             }
//             _ => {
//                 let mut regex = Regex::default();

//                 Regex::from_hir_internal(&hir, &mut regex);

//                 if regex.len() == 0 {
//                     None
//                 } else {
//                     Some(Branch::new(regex).into())
//                 }
//             }
            _ => Ok(None),
        }
    }
}

fn spanned_error(err: RError, span: Span) -> Error {
    Error::new(format!("{}\n\nIn this declaration:", err), span)
}