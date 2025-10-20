use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::Generator;
use crate::leaf::{Callback, Leaf, VariantKind};

impl Generator<'_> {
    /// This function generates the code responsible for calling user callbacks and returning
    /// an enum variant to the caller of [Logos::lex].
    /// Its return value is placed into the generated code whenever a match is encountered.
    /// The `leaf` parameter is the leaf node that was matched.
    pub fn generate_callback(&self, leaf: &Leaf) -> TokenStream {
        let name = self.name;
        let this = self.this;

        let callback_op = leaf.callback.as_ref().map(|cb| match cb {
            Callback::Label(ident) => quote!(#ident(lex)),
            Callback::Inline(inline_callback) => {
                let arg = &inline_callback.arg;
                let body = &inline_callback.body;

                quote! {{
                    let #arg = lex;
                    #body
                }}
            }
        });

        // Finally, based on both the kind of variant and the
        // presence / absence of a callback, implement the leaf.
        match (&leaf.kind, callback_op) {
            (VariantKind::Skip, None) => quote!(CallbackResult::Skip),
            (VariantKind::Skip, Some(cb)) => quote! {
                let cb_result = #cb;
                let srv = SkipRetVal::<'s, #this>::construct(cb_result);
                CallbackResult::from(srv)
            },
            (VariantKind::Unit(ident), None) => quote! {
                CallbackResult::Emit(#name::#ident)
            },
            (VariantKind::Unit(ident), Some(cb)) => quote! {
                let cb_result = #cb;
                CallbackRetVal::<'s, (), #this>::construct(cb_result, |()| #name::#ident)
            },
            (VariantKind::Value(ident, _), None) => quote! {
                let token = #name::#ident(lex.slice());
                CallbackResult::Emit(token)
            },
            (VariantKind::Value(ident, ret_type), Some(cb)) => quote! {
                let cb_result = #cb;
                CallbackRetVal::<'s, #ret_type, #this>::construct(cb_result, #name::#ident)
            },
        }
    }

    pub fn generate_leaf(&self) -> TokenStream {
        // This is the code block used to transition the lexer to a new state
        let restart_lex = self.state_transition(&self.graph.root());

        quote! {
            let action = _make_token(lex, offset, context);
            match action {
                CallbackResult::Emit(tok) => {
                    return Some(Ok(tok));
                },
                CallbackResult::Skip => {
                    lex.trivia();
                    offset = lex.offset();
                    context = 0usize;
                    #restart_lex
                },
                CallbackResult::Error(err) => {
                    return Some(Err(err));
                },
                CallbackResult::DefaultError => {
                    return Some(Err(_make_error(lex)));
                },
            }
        }
    }
}
