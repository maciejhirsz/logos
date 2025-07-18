use std::collections::HashMap;

use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use regex_automata::dfa::regex::Regex;
use syn::Ident;

use crate::error::Errors;
use crate::parser::definition::Literal;
use crate::pattern::Pattern;

pub struct Subpattern {
    name: Ident,
    pattern: String,
}

impl Subpattern {
    fn new(name: Ident, pattern_src: &Literal) -> Self {
        let pattern_str = pattern_src.escape();
        let flags = if pattern_src.unicode() { "u" } else { "-u" };
        let pattern = format!("(?{flags}:{pattern_str})");

        Self { name, pattern }
    }
}

#[derive(Default)]
pub struct Subpatterns {
    map: HashMap<String, Subpattern>,
}

lazy_static! {
    static ref SUBPATTERN_IDENT: Regex = Regex::new(r"[0-9a-zA-Z_]+").unwrap();
    static ref SUBPATTERN_GROUP: Regex = Regex::new(r"\(\?\&[0-9a-zA-Z_]+\)").unwrap();
}

impl Subpatterns {
    pub fn new(subpatterns: &Vec<(Ident, Literal)>, utf8_mode: bool, errors: &mut Errors) -> Self {
        let mut build = Self { map: HashMap::new() };
        for (name, pattern) in subpatterns {
            let name_string = name.to_string();

            let mut subpattern = Subpattern::new(name.clone(), pattern);

            if !SUBPATTERN_IDENT.is_match(&name_string) {
                errors.err(
                    format!("Invalid subpattern name: `{}`", name),
                    subpattern.name.span(),
                );
                continue;
            }

            if let Some(subst_pattern) = build.subst_subpatterns(&subpattern.pattern, pattern.span(), errors)
            {
                subpattern.pattern = subst_pattern
            } else {
                continue;
            };

            // Compile w/ unicode mode, since the top level flag will set it on or off anyway
            if let Err(msg) = Pattern::compile(&subpattern.pattern, utf8_mode, true) {
                errors.err(msg, pattern.span());
                continue;
            }

            if let Some(existing) = build.map.insert(name_string, subpattern) {
                errors
                    .err(
                        format!("Subpattern `{}` already exists", name),
                        name.span(),
                    )
                    .err("Previously assigned here", existing.name.span());
                continue;
            }
        }

        build
    }

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
                errors.err(format!("Subpattern `{}` not found", name), span);
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
