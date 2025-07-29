use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::Generator;
use crate::leaf::{Callback, Leaf, VariantKind};

impl Generator<'_> {
    pub fn generate_leaf(&self, leaf: &Leaf) -> TokenStream {
        let name = self.name;
        let this = self.this;

        let callback_op = leaf.callback.as_ref().map(|cb| match cb {
            Callback::Label(ident) => (ident.clone(), quote!()),
            Callback::Inline(inline_callback) => {
                let ident = quote!(callback);

                let arg = &inline_callback.arg;
                let body = &inline_callback.body;

                let ret = match &leaf.kind {
                    VariantKind::Unit(_) => quote! {
                        impl CallbackRetVal<'s, (), #this>
                    },
                    VariantKind::Value(_, ty) => quote! {
                        impl CallbackRetVal<'s, #ty, #this>
                    },
                    VariantKind::Skip => quote! {
                        impl SkipRetVal<'s, #this>
                    },
                };

                // TODO: shouldn't copy this callback code for every accept state?
                let decl = quote! {
                    #[inline]
                    fn callback<'s>(#arg: &mut _Lexer<'s>) -> #ret + use<'s> {
                        #body
                    }
                };
                (ident, decl)
            }
        });

        let constructor = match &leaf.kind {
            VariantKind::Unit(ident) => quote!(|()| #name::#ident),
            VariantKind::Value(ident, _ty) => quote!(#name::#ident),
            VariantKind::Skip => quote!(),
        };

        let start_ident = &self.idents[&self.graph.root()];
        let restart_lex = match self.config.use_state_machine_codegen {
            false => quote! { return #start_ident(lex, offset) },
            true => quote! { state = LogosState::#start_ident; },
        };
        let trivia = quote! {
            lex.trivia();
            offset = lex.offset();
            #restart_lex
        };

        let impl_callback_val = |decl, ty, cb_ident| {
            quote! {
                #decl
                let action = CallbackRetVal::<'s, #ty, #this>::construct(#cb_ident(lex), #constructor);
                match action {
                    CallbackResult::Emit(tok) => {
                        return Some(Ok(tok));
                    },
                    CallbackResult::Skip => {
                        #trivia
                    },
                    CallbackResult::Error(err) => {
                        return Some(Err(err));
                    },
                    CallbackResult::DefaultError => {
                        return Some(Err(make_error(lex)));
                    },
                }
            }
        };

        match (&leaf.kind, callback_op) {
            (VariantKind::Skip, None) => trivia,
            (VariantKind::Skip, Some((ident, decl))) => quote! {
                #decl
                let action = SkipRetVal::<'s, #this>::construct(#ident(lex));
                match action {
                    SkipResult::Skip => {
                        #trivia
                    },
                    SkipResult::Error(err) => {
                        return Some(Err(err));
                    },
                }
            },
            (VariantKind::Unit(ident), None) => quote! {
                return Some(Ok(#name::#ident));
            },
            (VariantKind::Unit(_ident), Some((cb_ident, decl))) => {
                impl_callback_val(decl, &quote!(()), cb_ident)
            }
            (VariantKind::Value(ident, _), None) => quote! {
                let token = #name::#ident(lex.slice());
                return Some(Ok(token));
            },
            (VariantKind::Value(_ident, ty), Some((cb_ident, decl))) => {
                impl_callback_val(decl, ty, cb_ident)
            }
        }
    }
}
