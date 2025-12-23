use std::{fmt, sync::LazyLock};

use crate::parser::Literal;

use regex_syntax::{
    hir::{Dot, Hir, HirKind},
    ParserBuilder,
};

static DOT_HIRS: LazyLock<[Hir; 6]> = LazyLock::new(|| {
    [
        Hir::dot(Dot::AnyChar),
        Hir::dot(Dot::AnyByte),
        Hir::dot(Dot::AnyByteExceptLF),
        Hir::dot(Dot::AnyCharExceptLF),
        Hir::dot(Dot::AnyByteExceptCRLF),
        Hir::dot(Dot::AnyCharExceptCRLF),
    ]
});

#[derive(Clone, Debug)]
pub struct Pattern {
    /// This field is only used to display #[regex] or #[token] in the display impl
    is_literal: bool,
    /// The original source literal for this pattern
    source: String,
    /// The parsed regex HIR for this pattern
    hir: Hir,
}

impl Pattern {
    /// Create a new pattern from a regex string source.
    ///
    /// # Arguments
    ///
    /// - `unicode`: whether the regex pattern should match bytes (false), or utf8 codepoints (true)
    /// - `ignore_case`: whether to set the `(?i)` flag for the entire pattern.
    pub fn compile(
        is_literal: bool,
        regex: &str,
        source: String,
        unicode: bool,
        ignore_case: bool,
    ) -> Result<Pattern, String> {
        // UTF-8 mode is disabled here so we can give prettier error messages
        // later in the compilation process. See logos_codegen/src/lib.rs for
        // the utf8 checking.
        let hir = ParserBuilder::new()
            .utf8(false)
            .unicode(unicode)
            .case_insensitive(ignore_case)
            .build()
            .parse(regex)
            .map_err(|err| format!("{err}"))?;

        Ok(Pattern {
            is_literal,
            source,
            hir,
        })
    }

    /// Create a pattern that matches a literal.
    ///
    /// This function avoids escaping by constructing an `Hir::literal` directly.
    pub fn compile_lit(source: &Literal) -> Result<Pattern, String> {
        let hir = match source {
            Literal::Utf8(lit_str) => Hir::literal(lit_str.value().as_bytes()),
            Literal::Bytes(lit_byte_str) => Hir::literal(lit_byte_str.value()),
        };

        Ok(Pattern {
            is_literal: true,
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
            HirKind::Literal(lit) => match std::str::from_utf8(&lit.0) {
                Ok(s) => 2 * s.chars().count(),
                Err(_) => 2 * lit.0.len(),
            },
            HirKind::Class(_) => 2,
            HirKind::Look(_) => 0,
            HirKind::Repetition(repetition) => {
                repetition.min as usize * Self::complexity(&repetition.sub)
            }
            HirKind::Capture(capture) => Self::complexity(&capture.sub),
            HirKind::Concat(hirs) => hirs.iter().map(Self::complexity).sum(),
            HirKind::Alternation(hirs) => hirs.iter().map(Self::complexity).min().unwrap_or(0),
        }
    }

    /// Return true if this pattern contains a non-greedy `.+` or `.*`
    pub fn check_for_greedy_all(&self) -> bool {
        Self::has_greedy_all(&self.hir)
    }

    fn has_greedy_all(hir: &Hir) -> bool {
        match hir.kind() {
            HirKind::Repetition(repetition) => {
                let is_dot = DOT_HIRS.contains(&repetition.sub);
                let is_unbounded = repetition.max.is_none();
                let is_greedy = repetition.greedy;

                is_dot && is_unbounded && is_greedy
            }
            HirKind::Empty => false,
            HirKind::Literal(_literal) => false,
            HirKind::Class(_class) => false,
            HirKind::Look(_look) => false,
            HirKind::Capture(capture) => Self::has_greedy_all(&capture.sub),
            HirKind::Concat(hirs) => hirs.iter().any(Self::has_greedy_all),
            HirKind::Alternation(hirs) => hirs.iter().any(Self::has_greedy_all),
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

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_literal {
            write!(f, "#[token({})]", self.source)
        } else {
            write!(f, "#[regex({})]", self.source)
        }
    }
}
