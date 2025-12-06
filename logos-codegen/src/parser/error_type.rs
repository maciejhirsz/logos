use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;
use syn::Ident;

use crate::leaf::Callback;
use crate::parser::nested::NestedValue;
use crate::parser::Parser;

pub struct ErrorType {
    pub ty: TokenStream,
    pub callback: Option<Callback>,
}

impl Default for ErrorType {
    fn default() -> Self {
        ErrorType {
            ty: quote::quote!(()),
            callback: None,
        }
    }
}

impl ErrorType {
    pub fn new(ty: TokenStream) -> Self {
        Self { ty, callback: None }
    }

    pub fn named_attr(&mut self, name: Ident, value: NestedValue, parser: &mut Parser) {
        match (name.to_string().as_str(), value) {
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
                        Unknown nested attribute: {unknown}\n\
                        \n\
                        Expected one of: callback\
                        "
                    ),
                    name.span(),
                );
            }
        }
    }

    pub fn span(&self) -> Span {
        self.ty.span()
    }
}
