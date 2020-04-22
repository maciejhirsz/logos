use proc_macro2::{Ident, TokenStream};
use syn::LifetimeDef;
use syn::spanned::Spanned;
use quote::quote;

use crate::error::Errors;

#[derive(Default)]
pub struct TypeParams {
    lifetime: bool,
    type_params: Vec<Ident>,
}

impl TypeParams {
    pub fn explicit_lifetime(&mut self, lt: LifetimeDef, errors: &mut Errors) {
        if self.lifetime {
            let span = lt.span();

            errors.err("Logos types can only have one lifetime can be set", span);
        }

        self.lifetime = true;
    }

    pub fn add_param(&mut self, param: Ident) {
        self.type_params.push(param);
    }

    pub fn generics(&self, errors: &mut Errors) -> Option<TokenStream> {
        if !self.lifetime && self.type_params.is_empty() {
            return None;
        }

        for ty in self.type_params.iter() {
            errors.err(
                format!(
                    "Generic type parameter without a concrete type\n\n\

                    Define a concrete type Logos can use: #[logos(type {} = Type)]",
                    ty,
                ),
                ty.span(),
            );
        }

        if self.lifetime {
            Some(quote!(<'s>))
        } else {
            None
        }
    }
}