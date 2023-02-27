use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use syn::{LitByteStr, LitStr};

use crate::error::{Errors, Result};
use crate::leaf::Callback;
use crate::mir::Mir;
use crate::parse::prelude::*;
use crate::parser::nested::NestedAssign;
use crate::parser::{IgnoreFlags, Subpatterns};

use super::ignore_flags::ascii_case::MakeAsciiCaseInsensitive;

pub struct Definition {
    pub literal: Literal,
    pub priority: Option<usize>,
    pub callback: Option<Callback>,
    pub ignore_flags: IgnoreFlags,
}

pub enum Literal {
    Utf8(LitStr),
    Bytes(LitByteStr),
}

impl Parse for Literal {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        let lit = stream.expect(Lit)?;
        let span = lit.span();

        match syn::parse2::<syn::Lit>(lit.into()) {
            Ok(syn::Lit::Str(string)) => Ok(Literal::Utf8(string)),
            Ok(syn::Lit::ByteStr(bytes)) => Ok(Literal::Bytes(bytes)),
            _ => Err(ParseError::new("Expected a &str or &[u8] slice", span)),
        }
    }
}

impl Definition {
    pub fn new(literal: Literal) -> Self {
        Definition {
            literal,
            priority: None,
            callback: None,
            ignore_flags: IgnoreFlags::Empty,
        }
    }

    pub fn named_attr(
        &mut self,
        stream: &mut ParseStream,
        errors: &mut Errors,
    ) -> Option<TokenStream> {
        type Handler = fn(
            &mut Definition,
            &mut Errors,
            stream: &mut ParseStream,
            Span,
        ) -> Result<(), ParseError>;

        let name: Ident = match stream.peek()? {
            TokenTree::Ident(_) => stream.parse().unwrap(),
            _ => return Some(stream.collect()),
        };

        // IMPORTANT: Keep these sorted alphabetically for binary search down the line
        static DEF_ATTRS: &[(&str, &str, Handler)] = &[
            (
                "callback",
                "Expected: callback = ...",
                |def, errors, stream, span| {
                    let NestedAssign { value } = stream.parse()?;

                    if let Some(previous) = def.callback.replace(value) {
                        errors
                            .err("Callback has been already set", span)
                            .err("Previous callback set here", previous.span());
                    }

                    Ok(())
                },
            ),
            (
                "ignore",
                "Expected: ignore(<flag>, ...)",
                |def, _, stream, _| {
                    if let TokenTree::Group(group) = stream.expect('(')? {
                        def.ignore_flags.parse_group(group.stream())?;
                    }

                    Ok(())
                },
            ),
            (
                "priority",
                "Expected: priority = <integer>",
                |def, _, stream, _| {
                    let NestedAssign { value } =
                        stream.parse::<NestedAssign<proc_macro2::Literal>>()?;

                    let prio = match value.to_string().parse() {
                        Ok(prio) => prio,
                        Err(_) => {
                            return Err(ParseError::new(
                                "Expected an unsigned integer",
                                value.span(),
                            ))
                        }
                    };

                    if def.priority.replace(prio).is_some() {
                        return Err(ParseError::new(
                            "Resetting previously set priority",
                            value.span(),
                        ));
                    }

                    Ok(())
                },
            ),
        ];

        match name.with_str(|name| DEF_ATTRS.binary_search_by_key(&name, |(n, _, _)| n)) {
            Ok(idx) => {
                let span = name.span();
                let (_, expected, handler) = DEF_ATTRS[idx];
                if let Err(err) = (handler)(self, errors, stream, span) {
                    errors.err(expected, span).push(err);
                }

                None
            }
            Err(_) => {
                let mut out = TokenStream::new();

                out.extend([TokenTree::Ident(name)]);
                out.extend(stream);

                Some(out)
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

    pub fn escape_regex(&self) -> Literal {
        match self {
            Literal::Utf8(string) => Literal::Utf8(LitStr::new(
                regex_syntax::escape(&string.value()).as_str(),
                self.span(),
            )),
            Literal::Bytes(bytes) => Literal::Bytes(LitByteStr::new(
                regex_syntax::escape(&bytes_to_regex_string(bytes.value())).as_bytes(),
                self.span(),
            )),
        }
    }

    pub fn to_mir(
        &self,
        subpatterns: &Subpatterns,
        ignore_flags: IgnoreFlags,
        errors: &mut Errors,
    ) -> Result<Mir> {
        let value = subpatterns.fix(self, errors);

        if ignore_flags.contains(IgnoreFlags::IgnoreAsciiCase) {
            match self {
                Literal::Utf8(_) => {
                    Mir::utf8(&value).map(MakeAsciiCaseInsensitive::make_ascii_case_insensitive)
                }
                Literal::Bytes(_) => Mir::binary_ignore_case(&value),
            }
        } else if ignore_flags.contains(IgnoreFlags::IgnoreCase) {
            match self {
                Literal::Utf8(_) => Mir::utf8_ignore_case(&value),
                Literal::Bytes(_) => Mir::binary_ignore_case(&value),
            }
        } else {
            match self {
                Literal::Utf8(_) => Mir::utf8(&value),
                Literal::Bytes(_) => Mir::binary(&value),
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
