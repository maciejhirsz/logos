use proc_macro2::{Ident, Span};
use syn::{LitStr, LitByteStr, spanned::Spanned};

use crate::leaf::Callback;
use crate::parser::nested::NestedValue;
use crate::parser::Parser;
use crate::mir::Mir;
use crate::error::Result;

pub struct Definition {
    pub literal: Literal,
    pub priority: Option<usize>,
    pub callback: Option<Callback>,
}

pub enum Literal {
    Utf8(LitStr),
    Bytes(LitByteStr),
}

impl Definition {
    pub fn new(literal: Literal) -> Self {
        Definition {
            literal,
            priority: None,
            callback: None,
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
            },
            ("priority", _) => {
                parser.err("Expected: priority = <integer>", name.span());
            },
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
            },
            ("callback", _) => {
                parser.err("Expected: callback = ...", name.span());
            },
            (unknown, _) => {
                parser.err(
                    format!(
                        "\
                        Unknown nested attribute: {}\n\n\

                        Expected one of: priority, callback\
                        ",
                        unknown
                    ),
                    name.span(),
                );
            }
        }
    }
}

impl Literal {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Literal::Utf8(string) => string.value().into_bytes(),
            Literal::Bytes(bytes) => bytes.value(),
        }
    }

    pub fn to_mir(&self) -> Result<Mir> {
        match self {
            Literal::Utf8(string) => Mir::utf8(&string.value()),
            Literal::Bytes(bytes) => {
                let source = bytes_to_regex_string(bytes.value());

                Mir::binary(&source)
            }
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Literal::Utf8(string) => string.span(),
            Literal::Bytes(bytes) => bytes.span(),
        }
    }
}

pub fn bytes_to_regex_string(bytes: Vec<u8>) -> String {
    if bytes.is_ascii() {
        unsafe {
            // Unicode values are prohibited, so we can't use
            // safe version of String::from_utf8
            //
            // We can, however, construct a safe ASCII string
            return String::from_utf8_unchecked(bytes);
        }
    }

    let mut string = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        if byte < 0x80 {
            string.push(byte as char);
        } else {
            static DIGITS: [u8; 16] = *b"0123456789abcdef";

            string.push_str(r"\x");
            string.push(DIGITS[(byte / 16) as usize] as char);
            string.push(DIGITS[(byte % 16) as usize] as char);
        }
    }

    string
}
