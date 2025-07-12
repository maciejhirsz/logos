use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::Generator;
use crate::leaf::{Callback, VariantKind, InlineCallback, Leaf};
use crate::util::MaybeVoid;

impl Generator<'_> {
    pub fn generate_leaf(&self, leaf: &Leaf) -> TokenStream {
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

                // TODO: shouldn't copy this callback code for every accept state?
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

        let trivia = quote! {
            lex.trivia();
            offset = lex.offset();
            state = START;
        };

        // TODO: default error is possibly taking one too many chars.
        // need to experiment with it

        match (&leaf.kind, callback_op) {
            (VariantKind::Skip, None) => trivia,
            (VariantKind::Skip, Some((ident, decl))) => quote! {
                #decl
                let action = SkipCallbackResult::<Self::Error>::from(#ident(lex));
                match action {
                    SkipCallbackResult::Skip => {
                        #trivia
                    },
                    SkipCallbackResult::Error(err) => {
                        return Some(Err(err));
                    },
                    SkipCallbackResult::DefaultError => {
                        return Some(Err(Self::Error::default()));
                    },
                }
            },
            (VariantKind::Unit(ident), None) => quote! {
                return Some(Ok(#name::#ident));
            },
            (VariantKind::Unit(ident), Some((cb_ident, decl))) => quote! {
                #decl
                let action = UnitVariantCallbackResult::<Self::Error>::from(#cb_ident(lex));
                match action {
                    UnitVariantCallbackResult::Emit => {
                        return Some(Ok(#name::#ident));
                    },
                    UnitVariantCallbackResult::Skip => {
                        #trivia
                    },
                    UnitVariantCallbackResult::Error(err) => {
                        return Some(Err(err));
                    },
                    UnitVariantCallbackResult::DefaultError => {
                        return Some(Err(Self::Error::default()));
                    },
                }
            },
            (VariantKind::Value(ident, _), None) => quote! {
                let token = #name::#ident(lex.slice());
                return Some(Ok(token));
            },
            (VariantKind::Value(ident, ty), Some((cb_ident, decl))) => quote! {
                #decl
                let action = FieldVariantCallbackResult::<#ty, Self::Error>::from(#cb_ident(lex));
                match action {
                    FieldVariantCallbackResult::Emit(val) => {
                        return Some(Ok(#name::#ident(val)));
                    },
                    FieldVariantCallbackResult::Skip => {
                        #trivia
                    },
                    FieldVariantCallbackResult::Error(err) => {
                        return Some(Err(err));
                    },
                    FieldVariantCallbackResult::DefaultError => {
                        return Some(Err(Self::Error::default()));
                    },
                }
            },
        }
    }
}
