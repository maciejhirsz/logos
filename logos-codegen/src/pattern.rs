use crate::parser::Literal;

use regex_syntax::{
    hir::{Hir, HirKind},
    Parser, ParserBuilder,
};

#[derive(Clone, Debug)]
pub struct Pattern {
    source: String,
    hir: Hir,
}

impl Pattern {
    pub fn compile(source: &str, utf8_mode: bool, unicode: bool, ignore_case: bool) -> Result<Pattern, String> {
        // TODO: don't create new parser every time
        let hir = ParserBuilder::new()
            .utf8(utf8_mode)
            .unicode(unicode)
            .case_insensitive(ignore_case)
            .build()
            .parse(source)
            .map_err(|err| format!("{}", err))?;

        Ok(Pattern { source: String::from(source), hir })
    }

    pub fn compile_lit(source: &Literal) -> Result<Pattern, String> {
        let hir = match source {
            Literal::Utf8(lit_str) => Hir::literal(lit_str.value().as_bytes()),
            Literal::Bytes(lit_byte_str) => Hir::literal(lit_byte_str.value()),
        };

        Ok(Pattern { source: source.token().to_string(), hir })
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
            }
            HirKind::Capture(capture) => Self::complexity(&*capture.sub),
            HirKind::Concat(hirs) => hirs.iter().map(Self::complexity).sum(),
            HirKind::Alternation(hirs) => hirs.iter().map(Self::complexity).max().unwrap_or(0),
        }
    }

    pub fn hir(&self) -> &Hir {
        &self.hir
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}
