use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::{Context, Generator};
use crate::leaf::{Callback, Leaf};
use crate::parser::SkipCallback;
use crate::util::MaybeVoid;

impl<'a> Generator<'a> {
    pub fn generate_leaf(&mut self, leaf: &Leaf, mut ctx: Context) -> TokenStream {
        let bump = ctx.bump();

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
                #bump
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
                    #bump

                    #[inline]
                    fn callback<'s>(#arg: &mut Lexer<'s>) -> #ret {
                        #body
                    }

                    callback(lex).construct(#constructor, lex);
                }
            }
            Some(Callback::SkipCallback(SkipCallback::Label(label))) => {
                quote! {
                    #bump
                    
                    trait SkipReturn {}
                    impl SkipReturn for () {}
                    impl SkipReturn for Skip {}

                    fn callback(lex: &mut Lexer) -> impl SkipReturn {
                        #label(lex)
                    }
                    
                    callback(lex);

                    lex.trivia();
                    #name::lex(lex);
                }
            }
            Some(Callback::SkipCallback(SkipCallback::Inline(inline))) => {
                let arg = &inline.arg;
                let body = &inline.body;

                quote! {
                    #bump

                    trait SkipReturn {}
                    impl SkipReturn for () {}
                    impl SkipReturn for Skip {}
                    
                    fn callback(#arg: &mut Lexer) -> impl SkipReturn {
                        #body
                    }

                    callback(lex);

                    lex.trivia();
                    #name::lex(lex);
                }
            }
            Some(Callback::Skip(_)) => {
                quote! {
                    #bump

                    lex.trivia();
                    #name::lex(lex);
                }
            }
            None if matches!(leaf.field, MaybeVoid::Void) => quote! {
                #bump
                lex.set(Ok(#name::#ident));
            },
            None => quote! {
                #bump
                let token = #name::#ident(lex.slice());
                lex.set(Ok(token));
            },
        }
    }
}
