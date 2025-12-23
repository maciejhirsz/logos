use std::collections::HashMap;
use std::sync::LazyLock;

use proc_macro2::Span;
use regex_automata::dfa::regex::Regex;
use syn::Ident;

use crate::error::Errors;
use crate::parser::definition::Literal;
use crate::pattern::Pattern;

/// This struct represents a logos subpattern, i.e.
/// `#[logos(subpattern my_subpattern = "regex")]`
/// These are regex subexpressions that can be referenced within
/// token regexes and other subpatterns to simplify complex expressions.
pub struct Subpattern {
    name: Ident,
    pattern: String,
}

impl Subpattern {
    /// Create a new subpattern with a given name and regex
    fn new(name: Ident, pattern_src: &Literal) -> Self {
        let pattern_str = pattern_src.escape(false);
        let flags = if pattern_src.unicode() { "u" } else { "-u" };
        let pattern = format!("(?{flags}:{pattern_str})");

        Self { name, pattern }
    }
}

#[derive(Default)]
pub struct Subpatterns {
    map: HashMap<String, Subpattern>,
}

static SUBPATTERN_IDENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-zA-Z_]+").unwrap());
static SUBPATTERN_GROUP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(\?\&[0-9a-zA-Z_]+\)").unwrap());

impl Subpatterns {
    /// Given a list of subpatterns, parse them sequentially and build a map of subpattern names to
    /// subpatterns.
    ///
    /// Returns any errors encountered through the `errors` argument. `utf8_mode` is passed to the
    /// regex compiler through a (?u:<src) flag. Each subsequent subpattern is allowed to reference
    /// previous subpatterns.
    pub fn new(subpatterns: &Vec<(Ident, Literal)>, utf8_mode: bool, errors: &mut Errors) -> Self {
        let mut build = Self {
            map: HashMap::new(),
        };
        for (name, pattern) in subpatterns {
            let name_string = name.to_string();

            let mut subpattern = Subpattern::new(name.clone(), pattern);

            if !SUBPATTERN_IDENT.is_match(&name_string) {
                errors.err(
                    format!("Invalid subpattern name: `{name}`"),
                    subpattern.name.span(),
                );
                continue;
            }

            if let Some(subst_pattern) =
                build.subst_subpatterns(&subpattern.pattern, pattern.span(), errors)
            {
                subpattern.pattern = subst_pattern
            } else {
                continue;
            };

            // Test compile the subpattern for better error messages
            // Compile w/ unicode mode, since the top level flag will set it on or off anyway
            match Pattern::compile(
                false,
                &subpattern.pattern,
                pattern.token().to_string(),
                true,
                false,
            ) {
                Err(msg) => {
                    errors.err(msg, pattern.span());
                    continue;
                }
                Ok(pat) => {
                    let utf8_pat = pat.hir().properties().is_utf8();
                    if utf8_mode && !utf8_pat {
                        errors.err(format!(concat!(
                            "UTF-8 mode is requested, but the subpattern {} = {} can match invalid utf8.\n",
                            "You can disable UTF-8 mode with #[logos(utf8 = false)]"
                        ), name, pat.source()), pattern.span());
                        continue;
                    }
                }
            }

            if let Some(existing) = build.map.insert(name_string, subpattern) {
                errors
                    .err(format!("Subpattern `{name}` already exists"), name.span())
                    .err("Previously assigned here", existing.name.span());
                continue;
            }
        }

        build
    }

    /// Given a new regex string, substitute existing subpatterns into the string.
    /// i.e. turn (?&my_subpattern) into the actual regex for "my_subpattern"
    /// Returns none if any non-existent subpatterns are referenced.
    pub fn subst_subpatterns(
        &self,
        pattern: &str,
        span: Span,
        errors: &mut Errors,
    ) -> Option<String> {
        let mut fragments = Vec::new();
        let mut was_error = false;

        let mut current_pos = 0;
        for group in SUBPATTERN_GROUP.find_iter(pattern) {
            // Add the text before the match
            if group.start() > current_pos {
                fragments.push(&pattern[current_pos..group.start()]);
            }

            // Extract the subpattern name
            let name = &pattern[group.start() + 3..group.end() - 1]; // Skip (?& and )
            if let Some(subpattern) = self.map.get(name) {
                fragments.push(&subpattern.pattern);
            } else {
                was_error = true;
                errors.err(format!("Subpattern `{name}` not found"), span);
            }
            current_pos = group.end();
        }

        if current_pos < pattern.len() {
            // Add the remaining text after the last match
            fragments.push(&pattern[current_pos..]);
        }

        if was_error {
            None
        } else {
            Some(fragments.join(""))
        }
    }
}
