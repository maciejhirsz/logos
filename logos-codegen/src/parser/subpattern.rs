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
    fn name_as_string(&self) -> String {
        self.name.to_string()
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
    pub fn add(&mut self, param: Ident, pattern: TokenStream, errors: &mut Errors) {
        let lit = match syn::parse2::<Literal>(pattern) {
            Ok(lit) => lit,
            Err(e) => {
                errors.err(e.to_string(), e.span());
                return;
            }
        };

        let mut subpattern = Subpattern {
            name: param.clone(),
            pattern: lit.escape(),
        };
        let name = subpattern.name_as_string();

        if !SUBPATTERN_IDENT.is_match(&name) {
            errors.err(
                format!("Invalid subpattern name: `{}`", name),
                subpattern.name.span(),
            );
            return;
        }

        if let Some(subst_pattern) = self.subst_subpatterns(&subpattern.pattern, lit.span(), errors)
        {
            subpattern.pattern = subst_pattern
        } else {
            return;
        };

        if let Err(msg) = Pattern::compile(&subpattern.pattern) {
            errors.err(msg, lit.span());
            return;
        }

        if let Some(existing) = self.map.insert(name, subpattern) {
            errors
                .err(
                    format!("Subpattern `{}` already exists", param),
                    param.span(),
                )
                .err("Previously assigned here", existing.name.span());
            return;
        }
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
