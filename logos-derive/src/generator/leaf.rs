use proc_macro2::TokenStream;
use quote::quote;

use crate::leaf::{Leaf, Callback};
use crate::generator::{Generator, Context};

impl<'a> Generator<'a> {
    pub fn generate_leaf(&mut self, leaf: &Leaf, mut ctx: Context) -> TokenStream {
        let bump = ctx.bump();

        let ident = &leaf.ident;
        let name = self.name;
        let this = self.this;

        let (ty, constructor) = match leaf.field.clone() {
            Some(ty) => (quote!(#ty), quote!(#name::#ident)),
            None => (quote!(()), quote!(|()| #name::#ident)),
        };

        match &leaf.callback {
            Callback::Label(callback) => quote! {
                #bump
                #callback(lex).construct(#constructor, lex);
            },
            Callback::Inline(arg, body) => quote! {
                #bump

                #[inline]
                fn callback<'s>(#arg: &mut Lexer<'s>) -> impl CallbackResult<'s, #ty, #this> {
                    #body
                }

                callback(lex).construct(#constructor, lex);
            },
            Callback::None if leaf.field.is_none() => quote! {
                #bump
                lex.set(#name::#ident);
            },
            Callback::None => quote! {
                #bump
                let token = #name::#ident(lex.slice());
                lex.set(token);
            },
        }
    }
}
