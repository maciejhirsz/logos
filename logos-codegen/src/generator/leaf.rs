use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::Generator;
use crate::leaf::{Callback, Leaf};
use crate::parser::SkipCallback;
use crate::util::MaybeVoid;

impl Generator<'_> {
    pub fn generate_leaf(&self, leaf: &Leaf) -> TokenStream {
        let ident = &leaf.ident;
        let name = self.name;
        let this = self.this;
        let ty = &leaf.field;

        let constructor = match leaf.field {
            MaybeVoid::Some(_) => quote!(#name::#ident),
            MaybeVoid::Void => quote!(|()| #name::#ident),
        };

        match &leaf.callback {
            Some(Callback::Label(callback)) => quote! {
                #callback(lex).construct(#constructor, lex);
            },
            Some(Callback::Inline(inline)) => {
                let arg = &inline.arg;
                let body = &inline.body;

                #[cfg(not(rust_1_82))]
                let ret = quote!(impl CallbackResult<'s, #ty, #this>);

                #[cfg(rust_1_82)]
                let ret = quote!(impl CallbackResult<'s, #ty, #this> + use<'s>);

                quote! {
                    #[inline]
                    fn callback<'s>(#arg: &mut Lexer<'s>) -> #ret {
                        #body
                    }

                    callback(lex).construct(#constructor, lex);
                }
            }
            Some(Callback::SkipCallback(SkipCallback::Label(label))) => {
                quote! {
                    #label(lex).construct_skip(lex);
                }
            }
            Some(Callback::SkipCallback(SkipCallback::Inline(inline))) => {
                let arg = &inline.arg;
                let body = &inline.body;

                #[cfg(not(rust_1_82))]
                let ret = quote!(impl SkipCallbackResult<'s, #this>);

                #[cfg(rust_1_82)]
                let ret = quote!(impl SkipCallbackResult<'s, #this> + use<'s>);

                quote! {
                    fn callback<'s>(#arg: &mut Lexer) -> #ret {
                        #body
                    }

                    callback(lex).construct_skip(lex);
                }
            }
            Some(Callback::Skip(_)) => {
                quote! {
                    println!("Trivia");
                    lex.trivia();
                    offset = lex.offset();
                    state = START;
                }
            }
            None if matches!(leaf.field, MaybeVoid::Void) => quote! {
                return Some(Ok(#name::#ident));
            },
            None => quote! {
                let token = #name::#ident(lex.slice());
                return Some(Ok(token));
            },
        }
    }
}
