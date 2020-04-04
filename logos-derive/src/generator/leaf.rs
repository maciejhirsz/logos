use proc_macro2::TokenStream;
use quote::quote;

use crate::leaf::Leaf;
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
            Leaf::Token { ident, callback, .. } => {
                let name = self.name;

                match callback {
                    Some(callback) => quote! {
                        #bump
                        lex.token = (#callback)(lex).construct(|()| #name::#ident);
                    },
                    None => quote! {
                        #bump
                        lex.token = #name::#ident;
                    },
                }
            },
        }
    }
}
