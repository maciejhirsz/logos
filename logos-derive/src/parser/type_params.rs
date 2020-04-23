use proc_macro2::{Ident, TokenStream, Span};
use syn::{Type, Path, Lifetime, LifetimeDef};
use syn::spanned::Spanned;
use quote::quote;

use crate::error::Errors;

#[derive(Default)]
pub struct TypeParams {
    lifetime: bool,
    type_params: Vec<(Ident, Option<Type>)>,
}

impl TypeParams {
    pub fn explicit_lifetime(&mut self, lt: LifetimeDef, errors: &mut Errors) {
        if self.lifetime {
            let span = lt.span();

            errors.err("Logos types can only have one lifetime can be set", span);
        }

        self.lifetime = true;
    }

    pub fn add(&mut self, param: Ident) {
        self.type_params.push((param, None));
    }

    pub fn set(&mut self, param: Ident, ty: TokenStream, errors: &mut Errors) {
        let ty = match syn::parse2::<Type>(ty) {
            Ok(mut ty) => {
                replace_lifetimes(&mut ty);
                ty
            },
            Err(err) => {
                errors.err(err.to_string(), err.span());
                return;
            },
        };

        match self.type_params.iter_mut().find(|(name, _)| *name == param) {
            Some((_, slot)) => {
                if let Some(previous) = slot.replace(ty) {
                    errors
                        .err(
                            format!("{} can only have one type assigned to it", param),
                            param.span(),
                        )
                        .err("Previously assigned here", previous.span());
                }
            },
            None => {
                errors.err(
                    format!("{} is not a declared type parameter", param),
                    param.span(),
                );
            }
        }
    }

    pub fn find(&self, path: &Path) -> Option<Type> {
        for (ident, ty) in &self.type_params {
            if path.is_ident(ident) {
                return ty.clone();
            }
        }

        None
    }

    pub fn generics(&self, errors: &mut Errors) -> Option<TokenStream> {
        if !self.lifetime && self.type_params.is_empty() {
            return None;
        }

        let mut generics = Vec::new();

        if self.lifetime {
            generics.push(quote!('s));
        }

        for (ty, replace) in self.type_params.iter() {
            match replace {
                Some(ty) => generics.push(quote!(#ty)),
                None => {
                    errors.err(
                        format!(
                            "Generic type parameter without a concrete type\n\n\

                            Define a concrete type Logos can use: #[logos(type {} = Type)]",
                            ty,
                        ),
                        ty.span(),
                    );
                }
            }
        }

        if generics.is_empty() {
            None
        } else {
            Some(quote!(<#(#generics),*>))
        }
    }
}

pub fn replace_lifetimes(ty: &mut Type) {
    replace_types(ty, &mut replace_lifetime)
}

pub fn replace_lifetime(ty: &mut Type) {
    use syn::{PathArguments, GenericArgument};

    match ty {
        Type::Path(p) => {
            p.path.segments
                .iter_mut()
                .filter_map(|segment| match &mut segment.arguments {
                    PathArguments::AngleBracketed(ab) => Some(ab),
                    _ => None,
                })
                .flat_map(|ab| ab.args.iter_mut())
                .for_each(|arg| {
                    match arg {
                        GenericArgument::Lifetime(lt) => {
                            *lt = Lifetime::new("'s", lt.span());
                        },
                        _ => (),
                    }
                });
        },
        Type::Reference(r) => {
            let span = match r.lifetime.take() {
                Some(lt) => lt.span(),
                None => Span::call_site(),
            };

            r.lifetime = Some(Lifetime::new("'s", span));
        },
        _ => (),
    }
}

pub fn replace_types(ty: &mut Type, f: &mut impl FnMut(&mut Type)) {
    f(ty);
    match ty {
        Type::Array(array) => replace_types(&mut array.elem, f),
        Type::BareFn(bare_fn) => {
            for input in &mut bare_fn.inputs {
                replace_types(&mut input.ty, f);
            }
            if let syn::ReturnType::Type(_, ty) = &mut bare_fn.output {
                replace_types(ty, f);
            }
        }
        Type::Group(group) => replace_types(&mut group.elem, f),
        Type::Paren(paren) => replace_types(&mut paren.elem, f),
        Type::Path(path) => replace_path(&mut path.path, f),
        Type::Ptr(p) => {
            replace_types(&mut p.elem, f)
        },
        Type::Reference(r) => {
            replace_types(&mut r.elem, f)
        },
        Type::Slice(slice) => replace_types(&mut slice.elem, f),
        Type::TraitObject(object) => object.bounds.iter_mut().for_each(|bound| {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                replace_path(&mut trait_bound.path, f);
            }
        }),
        Type::Tuple(tuple) => tuple.elems.iter_mut().for_each(|elem| replace_types(elem, f)),
        _ => (),
    }
}

fn replace_path(path: &mut Path, f: &mut impl FnMut(&mut Type)) {
    for segment in &mut path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => (),
            syn::PathArguments::AngleBracketed(args) => for arg in &mut args.args {
                match arg {
                    syn::GenericArgument::Type(ty) => {
                        replace_types(ty, f);
                    },
                    syn::GenericArgument::Binding(bind) => {
                        replace_types(&mut bind.ty, f);
                    },
                    _ => (),
                }
            },
            syn::PathArguments::Parenthesized(args) => {
                for arg in &mut args.inputs {
                    replace_types(arg, f);
                }
                if let syn::ReturnType::Type(_, ty) = &mut args.output {
                    replace_types(ty, f);
                }
            }
        }
    }
}
