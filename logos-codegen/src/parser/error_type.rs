use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;
use syn::Ident;

use crate::leaf::Callback;
use crate::parser::nested::NestedValue;
use crate::parser::Parser;
use crate::util::MaybeVoid;

pub struct ErrorType {
    pub ty: TokenStream,
    pub callback: Option<Callback>,
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
                        Unknown nested attribute: {}\n\
                        \n\
                        Expected one of: callback\
                        ",
                        unknown
                    ),
                    name.span(),
                );
            }
        }
    }

    pub fn unwrap(opt: Option<Self>) -> (MaybeVoid, Option<Callback>) {
        if let Some(Self { ty, callback }) = opt {
            (MaybeVoid::Some(ty), callback)
        } else {
            (MaybeVoid::Void, None)
        }
    }

    pub fn span(&self) -> Span {
        self.ty.span()
    }
}
