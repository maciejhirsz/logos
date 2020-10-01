use proc_macro2::TokenStream;
use quote::quote;

use crate::leaf::{Leaf, Callback};
use crate::generator::{Generator, Context};
use crate::util::MaybeVoid;

impl<'a> Generator<'a> {
    pub fn generate_leaf(&mut self, leaf: &Leaf, mut ctx: Context) -> TokenStream {
        let bump = ctx.bump();

        let ident = &leaf.ident;
        let name = self.name;
        let this = self.this;
        let ty = &leaf.field;

        let constructor = match leaf.field {
            MaybeVoid::Some(_) => quote!(Ok(#name::#ident)),
            MaybeVoid::Void => quote!(|()| Ok(#name::#ident)),
        };

        match &leaf.callback {
            Some(Callback::Label(callback)) => quote! {
                #bump
                #callback(lex).construct(#constructor, lex);
            },
            Some(Callback::Inline(inline)) => {
                let arg = &inline.arg;
                let body = &inline.body;

                quote! {
                    #bump

                    #[inline]
                    fn callback<'s>(#arg: &mut Lexer<'s>) -> impl CallbackResult<'s, #ty, #this> {
                        #body
                    }

                    callback(lex).construct(#constructor, lex);
                }
            },
            None if matches!(leaf.field, MaybeVoid::Void) => quote! {
                #bump
                lex.set(Ok(#name::#ident));
            },
            None => quote! {
                #bump
                let token = Ok(#name::#ident(lex.slice()));
                lex.set(token);
            },
        }
    }
}
