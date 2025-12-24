use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::{quote_spanned, ToTokens, TokenStreamExt};
use std::borrow::Cow;

#[derive(Default)]
pub struct Errors {
    collected: Vec<SpannedError>,
}

impl Errors {
    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Self
    where
        M: Into<Cow<'static, str>>,
    {
        self.collected.push(SpannedError {
            message: message.into(),
            span,
        });

        self
    }

    pub fn render(self) -> Option<TokenStream> {
        let errors = self.collected;

        // Each of the SpannedErrors get rendered into a compile_error!()
        // invocation (see ToTokens implementation below).
        match errors.len() {
            0 => None,
            _ => Some(quote! {
                fn _logos_derive_compile_errors() {
                    #(#errors)*
                }

                unimplemented!()
            }),
        }
    }
}

#[derive(Debug)]
pub struct SpannedError {
    message: Cow<'static, str>,
    span: Span,
}

impl ToTokens for SpannedError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let message = &*self.message;

        tokens.append_all(quote_spanned!(self.span => {
            compile_error!(#message)
        }))
    }
}
