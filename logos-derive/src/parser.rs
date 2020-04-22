use beef::lean::Cow;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, spanned::Spanned};

use crate::util;
use crate::error::Errors;
use crate::parsers::{AttributeParser, Nested, NestedValue};

pub struct Parser {
    pub errors: Errors,
    extras: Option<TokenStream>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            errors: Errors::new(),
            extras: None,
        }
    }

    pub fn extras(&mut self) -> TokenStream {
        self.extras.take().unwrap_or_else(|| quote!(()))
    }

    pub fn try_parse_logos(&mut self, attr: &Attribute) {
        if !attr.path.is_ident("logos") {
            return;
        }

        let nested = util::read_attr("logos", attr, &mut self.errors);

        for nested in AttributeParser::new(nested.clone()) {
            let (name, value) = match nested {
                Nested::Named(name, value) => (name, value),
                Nested::Unexpected(unexpected) => {
                    self.err("Unexpected tokens in attribute", unexpected.span());
                    continue;
                },
                Nested::Unnamed(unnamed) => {
                    self.err("Expected a named nested attribute", unnamed.span());
                    continue;
                },
            };

            match (name.to_string().as_str(), value) {
                ("extras", NestedValue::Assign(value)) => {
                    let span = value.span();

                    if let Some(previous) = self.extras.replace(value) {
                        self.err("Extras can be defined only once", span)
                            .err("Previous definition here", previous.span());
                    }
                },
                ("extras", _) => {
                    self.err("Expected = after this token", name.span());
                }
                ("type", _) => (),
                ("trivia", _) => {
                    // TODO: Remove in future versions
                    self.err(
                        "\
                        trivia are no longer supported.\n\n\

                        For help with migration see release notes: \
                        https://github.com/maciejhirsz/logos/releases\
                        ",
                        name.span(),
                    );
                },
                (unknown, _) => {
                    self.err(
                        format!("Unknown nested attribute {}", unknown),
                        name.span(),
                    );
                }
            }
        }
    }

    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Errors
    where
        M: Into<Cow<'static, str>>,
    {
        self.errors.err(message, span)
    }
}