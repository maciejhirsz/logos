use proc_macro2::TokenStream;
use quote::quote;

use crate::leaf::{Leaf, Callback};
use crate::generator::{Generator, Context};

impl<'a> Generator<'a> {
    pub fn generate_leaf(&mut self, leaf: &Leaf, mut ctx: Context) -> TokenStream {
        let bump = ctx.bump();

        match leaf {
            Leaf::Trivia => {
                let root = self.goto(self.root, Context::default());

                quote! {
                    #bump
                    lex.trivia();
                    return #root;
                }
            },
            Leaf::Token { ident, callback, field, .. } => {
                let name = self.name;

                let (ty, constructor) = match field {
                    Some(ty) => (quote!(#ty), quote!(#name::#ident)),
                    None => (quote!(()), quote!(|()| #name::#ident)),
                };

                match callback {
                    Callback::Label(callback) => quote! {
                        #bump
                        lex.token = #callback(lex).construct(#constructor);
                    },
                    Callback::Inline(arg, body) => quote! {
                        #bump

                        #[inline]
                        fn __callback<'s>(#arg: &mut Lexer<'s>) -> impl CallbackResult<#ty> {
                            #body
                        }

                        lex.token = __callback(lex).construct(#constructor);
                    },
                    Callback::None => quote! {
                        #bump
                        lex.token = #name::#ident;
                    },
                }
            },
        }
    }
}
