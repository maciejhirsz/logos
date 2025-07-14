use proc_macro2::Ident;
use syn::spanned::Spanned;

use crate::leaf::Callback;
use crate::parser::nested::NestedValue;
use crate::parser::{Literal, Parser};

pub struct Skip {
    pub literal: Literal,
    pub callback: Option<Callback>,
    pub priority: Option<usize>,
}

impl Skip {
    pub fn new(literal: Literal) -> Self {
        Self {
            literal,
            callback: None,
            priority: None,
        }
    }

    pub fn named_attr(&mut self, name: Ident, value: NestedValue, parser: &mut Parser) {
        match (name.to_string().as_str(), value) {
            ("priority", NestedValue::Assign(tokens)) => {
                let prio = match tokens.to_string().parse() {
                    Ok(prio) => prio,
                    Err(_) => {
                        parser.err("Expected an unsigned integer", tokens.span());
                        return;
                    }
                };

                if self.priority.replace(prio).is_some() {
                    parser.err("Resetting previously set priority", tokens.span());
                }
            }
            ("priority", _) => {
                parser.err("Expected: priority = <integer>", name.span());
            }
            ("callback", NestedValue::Assign(tokens)) => {
                let span = tokens.span();
                let callback = match parser.parse_callback(tokens) {
                    Some(callback) => callback,
                    None => {
                        parser.err("Not a valid callback", span);
                        return;
                    }
                };

                if let Some(previous) = self.callback.replace(callback) {
                    parser
                        .err(
                            "Callback has been already set",
                            span.join(name.span()).unwrap(),
                        )
                        .err("Previous callback set here", previous.span());
                }
            }
            ("callback", _) => {
                parser.err("Expected: callback = ...", name.span());
            }
            (unknown, _) => {
                parser.err(
                    format!(
                        "\
                        Unknown nested attribute: {}\n\
                        \n\
                        Expected: callback or priority\
                        ",
                        unknown
                    ),
                    name.span(),
                );
            }
        }
    }

    pub fn into_callback(self) -> Option<Callback> {
        self.callback
    }
}

