use std::ops::Index;

use regex_automata::dfa::dense::DFA;
use regex_automata::nfa::thompson::NFA;

use crate::leaf::Leaf;

use regex_syntax::{hir::{Hir, HirKind}, Parser};

#[derive(Clone, Debug)]
pub struct Pattern {
    hir: Hir,
}

impl Pattern {
    pub fn compile(source: &str) -> Result<Pattern, String> {
        // TODO: Can support non-utf8 patterns through this parser
        // TODO: don't create new parser every time
        let hir = Parser::new().parse(source)
            .map_err(|err| format!("{}", err))?;

        Ok(Pattern { hir })
    }

    pub fn priority(&self) -> usize {
        Self::complexity(&self.hir)
    }

    fn complexity(hir: &Hir) -> usize {
        match hir.kind() {
            HirKind::Empty => 0,
            // TODO: complexity will be slightly different for unicode patterns
            HirKind::Literal(literal) => 2 * literal.0.len(),
            HirKind::Class(_) => 2,
            // TODO:
            HirKind::Look(_) => unimplemented!("Lookarounds are not implemented"),
            HirKind::Repetition(repetition) => {
                repetition.min as usize * Self::complexity(&*repetition.sub)
            },
            HirKind::Capture(capture) => Self::complexity(&*capture.sub),
            HirKind::Concat(hirs) => hirs.iter().map(Self::complexity).sum(),
            HirKind::Alternation(hirs) => hirs.iter().map(Self::complexity).max().unwrap_or(0),
        }
    }

    pub fn hir(&self) -> &Hir {
        &self.hir
    }
}
