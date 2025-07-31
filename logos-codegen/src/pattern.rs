use crate::parser::Literal;

use regex_syntax::{
    hir::{Hir, HirKind},
    ParserBuilder,
};

#[derive(Clone, Debug)]
pub struct Pattern {
    source: String,
    hir: Hir,
}

impl Pattern {
    /// Create a new pattern from a regex string source.
    /// - `utf8_mode` determines if the pattern should refuse to match invalid utf8 sequences
    /// - `unicode` determines if the regex pattern should match bytes (false) or utf8 codepoints
    /// (true)
    /// - `ignore_case` sets the (?i) flag for the entire pattern.
    ///
    /// There are some cases where the value of `utf8_mode` and `unicode` may not match. For
    /// example, if your input is a `[u8]`, but you want to match specific parts of it as UTF-8, you
    /// would have only `unicode` set.
    pub fn compile(
        source: &str,
        utf8_mode: bool,
        unicode: bool,
        ignore_case: bool,
    ) -> Result<Pattern, String> {
        let hir = ParserBuilder::new()
            .utf8(utf8_mode)
            .unicode(unicode)
            .case_insensitive(ignore_case)
            .build()
            .parse(source)
            .map_err(|err| format!("{}", err))?;

        Ok(Pattern {
            source: String::from(source),
            hir,
        })
    }

    /// Create a pattern that matches a literal.
    ///
    /// This function avoids escaping by constructing an Hir literal directly.
    pub fn compile_lit(source: &Literal) -> Result<Pattern, String> {
        let hir = match source {
            Literal::Utf8(lit_str) => Hir::literal(lit_str.value().as_bytes()),
            Literal::Bytes(lit_byte_str) => Hir::literal(lit_byte_str.value()),
        };

        Ok(Pattern {
            source: source.token().to_string(),
            hir,
        })
    }

    /// Get the default priority for a pattern
    pub fn priority(&self) -> usize {
        Self::complexity(&self.hir)
    }

    fn complexity(hir: &Hir) -> usize {
        match hir.kind() {
            HirKind::Empty => 0,
            // The old logos behavior used the 2 * the number of characters for unicode literals,
            // but the regex crate's hir doesn't differentiate between them, so it will report
            // slightly higher complexity for non-ascii unicode patterns.
            HirKind::Literal(literal) => 2 * literal.0.len(),
            HirKind::Class(_) => 2,
            // TODO: better error handling
            HirKind::Look(_) => unimplemented!("Lookarounds are not implemented"),
            HirKind::Repetition(repetition) => {
                repetition.min as usize * Self::complexity(&*repetition.sub)
            }
            HirKind::Capture(capture) => Self::complexity(&*capture.sub),
            HirKind::Concat(hirs) => hirs.iter().map(Self::complexity).sum(),
            HirKind::Alternation(hirs) => hirs.iter().map(Self::complexity).max().unwrap_or(0),
        }
    }

    /// Get a reference to a pattern's Hir
    pub fn hir(&self) -> &Hir {
        &self.hir
    }

    /// Get a reference to the original source string of the pattern
    pub fn source(&self) -> &str {
        &self.source
    }
}
