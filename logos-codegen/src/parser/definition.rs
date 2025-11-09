use std::fmt::Write;

use proc_macro2::{Ident, Span};
use syn::{spanned::Spanned, LitByteStr, LitStr};

use crate::leaf::Callback;
use crate::parser::nested::NestedValue;
use crate::parser::{IgnoreFlags, Parser};

pub struct Definition {
    pub literal: Literal,
    pub priority: Option<usize>,
    pub callback: Option<Callback>,
    pub allow_greedy: Option<bool>,
    pub ignore_flags: IgnoreFlags,
}

pub enum Literal {
    Utf8(LitStr),
    Bytes(LitByteStr),
}

impl Literal {
    /// Escape this literal into a regex_syntax compatible pattern string.
    /// - `literal`: if true, escape any metacharacters the pattern so that it matches literally.
    ///   This is necessary so that literal byte strings can be implemented properly.
    pub fn escape(&self, literal: bool) -> String {
        match self {
            Literal::Utf8(lit_str) if literal => regex_syntax::escape(&lit_str.value()),
            Literal::Utf8(lit_str) => lit_str.value(),
            Literal::Bytes(lit_byte_str) => {
                let mut pattern = String::new();
                for byte in lit_byte_str.value() {
                    if byte <= 127 {
                        if literal {
                            let buf = [byte];
                            let s = std::str::from_utf8(&buf).expect("Ascii is always valid utf8");
                            regex_syntax::escape_into(s, &mut pattern);
                            Ok(())
                        } else {
                            write!(pattern, "{}", byte as char)
                        }
                    } else {
                        write!(pattern, "\\x{byte:02X}")
                    }
                    .expect("Writing to a string should not fail");
                }
                pattern
            }
        }
    }

    pub fn token(&self) -> proc_macro2::Literal {
        match self {
            Literal::Utf8(lit_str) => lit_str.token(),
            Literal::Bytes(lit_byte_str) => lit_byte_str.token(),
        }
    }

    pub fn unicode(&self) -> bool {
        matches!(self, Literal::Utf8(_))
    }
}

impl Definition {
    pub fn new(literal: Literal) -> Self {
        Definition {
            literal,
            priority: None,
            callback: None,
            allow_greedy: None,
            ignore_flags: IgnoreFlags::default(),
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
            ("ignore", NestedValue::Group(tokens)) => {
                self.ignore_flags.parse_group(name, tokens, parser);
            }
            ("ignore", _) => {
                parser.err("Expected: ignore(<flag>, ...)", name.span());
            }
            ("allow_greedy", NestedValue::Assign(tokens)) => {
                let allow = match tokens.to_string().parse() {
                    Ok(allow) => allow,
                    Err(_) => {
                        parser.err("Expected `true` or `false`", tokens.span());
                        return;
                    }
                };

                if self.allow_greedy.replace(allow).is_some() {
                    parser.err("Resetting previously set allow_greedy", tokens.span());
                }
            }
            ("allow_greedy", _) => {
                parser.err("Expected: allow_greedy = ...", name.span());
            }
            (unknown, _) => {
                parser.err(
                    format!(
                        "\
                        Unknown nested attribute: {unknown}\n\
                        \n\
                        Expected one of: priority, callback\
                        "
                    ),
                    name.span(),
                );
            }
        }
    }
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Utf8(string) => string.span(),
            Literal::Bytes(bytes) => bytes.span(),
        }
    }
}

impl syn::parse::Parse for Literal {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let la = input.lookahead1();
        if la.peek(LitStr) {
            Ok(Literal::Utf8(input.parse()?))
        } else if la.peek(LitByteStr) {
            Ok(Literal::Bytes(input.parse()?))
        } else {
            Err(la.error())
        }
    }
}
