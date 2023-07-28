use std::convert::TryFrom;

use lazy_static::lazy_static;
use regex_syntax::hir::{Dot, Hir, HirKind, Repetition};
use regex_syntax::ParserBuilder;

pub use regex_syntax::hir::{Class, ClassUnicode, Literal};

use crate::error::{Error, Result};

lazy_static! {
    /// DOT regex that matches utf8 only.
    static ref DOT_UTF8: Hir = Hir::dot(Dot::AnyChar);

    /// DOT regex that matches any byte.
    static ref DOT_BYTES: Hir = Hir::dot(Dot::AnyByte);
}

/// Middle Intermediate Representation of the regex, built from
/// `regex_syntax`'s `Hir`. The goal here is to strip and canonicalize
/// the tree, so that we don't have to do transformations later on the
/// graph, with the potential of running into looping references.
#[derive(Clone, Debug)]
pub enum Mir {
    Empty,
    Loop(Box<Mir>),
    Maybe(Box<Mir>),
    Concat(Vec<Mir>),
    Alternation(Vec<Mir>),
    Class(Class),
    Literal(char),
}

impl Mir {
    pub fn utf8(source: &str) -> Result<Mir> {
        Mir::try_from(ParserBuilder::new().build().parse(source)?)
    }

    pub fn utf8_ignore_case(source: &str) -> Result<Mir> {
        Mir::try_from(
            ParserBuilder::new()
                .case_insensitive(true)
                .build()
                .parse(source)?,
        )
    }

    pub fn binary(source: &str) -> Result<Mir> {
        Mir::try_from(
            ParserBuilder::new()
                .utf8(false)
                .unicode(false)
                .build()
                .parse(source)?,
        )
    }

    pub fn binary_ignore_case(source: &str) -> Result<Mir> {
        Mir::try_from(
            ParserBuilder::new()
                .utf8(false)
                .unicode(false)
                .case_insensitive(true)
                .build()
                .parse(source)?,
        )
    }

    pub fn priority(&self) -> usize {
        match self {
            Mir::Empty | Mir::Loop(_) | Mir::Maybe(_) => 0,
            Mir::Concat(concat) => concat.iter().map(Mir::priority).sum(),
            Mir::Alternation(alt) => alt.iter().map(Mir::priority).min().unwrap_or(0),
            Mir::Class(_) => 1,
            Mir::Literal(_) => 2,
        }
    }
}

impl TryFrom<Hir> for Mir {
    type Error = Error;

    fn try_from(hir: Hir) -> Result<Mir> {
        match hir.into_kind() {
            HirKind::Empty => Ok(Mir::Empty),
            HirKind::Concat(concat) => {
                let mut out = Vec::with_capacity(concat.len());

                fn extend(mir: Mir, out: &mut Vec<Mir>) {
                    match mir {
                        Mir::Concat(nested) => {
                            for child in nested {
                                extend(child, out);
                            }
                        }
                        mir => out.push(mir),
                    }
                }

                for hir in concat {
                    extend(Mir::try_from(hir)?, &mut out);
                }

                Ok(Mir::Concat(out))
            }
            HirKind::Alternation(alternation) => {
                let alternation = alternation
                    .into_iter()
                    .map(Mir::try_from)
                    .collect::<Result<_>>()?;

                Ok(Mir::Alternation(alternation))
            }
            HirKind::Literal(literal) => {
                let s = std::str::from_utf8(&*literal.0).unwrap();
                let mut chars = s.chars().map(Mir::Literal).peekable();
                let c = chars.next().expect("a literal cannot be empty");
                if chars.peek().is_some() {
                    Ok(Mir::Concat(std::iter::once(c).chain(chars).collect()))
                } else {
                    Ok(c)
                }
            }
            HirKind::Class(class) => Ok(Mir::Class(class)),
            HirKind::Repetition(repetition) => {
                let Repetition {
                    min,
                    max,
                    sub,
                    greedy,
                } = repetition;

                if !greedy {
                    return Err("#[regex]: non-greedy parsing is currently unsupported.".into());
                }

                let is_dot = if sub.properties().is_utf8() {
                    *sub == *DOT_UTF8
                } else {
                    *sub == *DOT_BYTES
                };

                let sub: Mir = Mir::try_from(*sub)?;
                match (min, max) {
                    (0 | 1, None) if is_dot => {
                        Err(
                            "#[regex]: \".+\" and \".*\" patterns will greedily consume \
                            the entire source till the end as Logos does not allow \
                            backtracking. If you are looking to match everything until \
                            a specific character, you should use a negative character \
                            class. E.g., use regex r\"'[^']*'\" to match anything in \
                            between two quotes. Read more about that here: \
                            https://github.com/maciejhirsz/logos/issues/302#issuecomment-1521342541."
                            .into()
                        )
                    }
                    // ZeroOrOne
                    (0, Some(1)) => Ok(Mir::Maybe(Box::new(sub))),
                    // ZeroOrMore
                    (0, None) => Ok(Mir::Loop(Box::new(sub))),
                    // OneOrMore
                    (1, None) => Ok(Mir::Concat(vec![sub.clone(), Mir::Loop(Box::new(sub))])),
                    // Exactly
                    (n, Some(m)) if n == m => Ok(Mir::Concat(
                        std::iter::repeat(sub).take(n as usize).collect(),
                    )),
                    // AtLeast
                    (n, None) => Ok(Mir::Concat(
                        (std::iter::repeat(sub.clone()).take(n as usize))
                            .chain([Mir::Loop(Box::new(sub))])
                            .collect(),
                    )),
                    // Bounded
                    (n, Some(m)) => Ok(Mir::Concat(
                        (std::iter::repeat(sub.clone()).take(n as usize))
                            .chain(std::iter::repeat(Mir::Maybe(Box::new(sub))).take((n..m).len()))
                            .collect(),
                    )),
                }
            }
            HirKind::Look(_) => {
                Err("#[regex]: lookahead and lookbehind are currently unsupported.".into())
            }
            HirKind::Capture(capture) => Mir::try_from(*capture.sub),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mir;

    #[test]
    fn priorities() {
        let regexes = [
            ("[a-z]+", 1),
            ("a|b", 1),
            ("a|[b-z]", 1),
            ("(foo)+", 6),
            ("foobar", 12),
            ("(fooz|bar)+qux", 12),
        ];

        for (regex, expected) in regexes.iter() {
            let mir = Mir::utf8(regex).unwrap();
            assert_eq!(mir.priority(), *expected);
        }
    }
}
