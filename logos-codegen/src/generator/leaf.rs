use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::Generator;
use crate::leaf::{Callback, CallbackKind, InlineCallback, Leaf};
use crate::util::MaybeVoid;

impl Generator<'_> {
    pub fn generate_leaf(&self, leaf: &Leaf) -> TokenStream {
        let ident = &leaf.ident;
        let name = self.name;
        let this = self.this;

        let callback_op = leaf.callback.as_ref().map(|cb| match cb {
            Callback::Label(ident) => (
                ident.clone(),
                quote!(),
            ),
            Callback::Inline(inline_callback) => {
                let ident = quote!(callback);

                let arg = &inline_callback.arg;
                let body = &inline_callback.body;

                // TODO: shouldn't copy this code?
                let decl = quote! {
                    #[inline]
                    fn callback<'s>(#arg: &mut Lexer<'s>)
                        -> Option<Result<Self, Self::Error>>
                    {
                        #body
                    }
                };
                (ident, decl)
            },
        });

        match (&leaf.kind, callback_op) {
            (CallbackKind::Skip, Some((ident, decl))) => quote! {
                #decl
                let action = SkipCallbackResult::from(#ident(lex));
                match action {
                    SkipCallbackResult::Skip => {
                        lex.trivia();
                        offset = lex.offset();
                        state = START;
                    },
                    SkipCallbackResult::Error(err) => {
                        return Some(err.into());
                    },
                    SkipCallbackResult::DefaultError => {
                        lex.error(offset);
                        return Some(Err(Self::Error::default()));
                    },
                }
            },
            (CallbackKind::Skip, None) => quote! {
                lex.trivia();
                offset = lex.offset();
                state = START;
            },
            (CallbackKind::Unit, None) => quote! {
                return Some(Ok(#name::#ident));
            },
            (CallbackKind::Value(_), None) => quote! {
                let token = #name::#ident(lex.slice());
                return Some(Ok(token));
            },
            _ => todo!(),
        }
    }
}
