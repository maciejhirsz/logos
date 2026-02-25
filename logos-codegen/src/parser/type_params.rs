use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Lifetime, LifetimeParam, Path, Type};

use crate::error::Errors;

#[derive(Default)]
pub enum SourceLifetime {
    /// Uses 's as the source and fixes all lifetimes to the source
    #[default]
    Implicit,
    /// Generate a fresh lifetime to use as the source
    Fresh(Span),
    /// Use the lifetime from the parameter list matching the provided lifetime
    Named(Lifetime),
}

pub struct TypeParams {
    lifetime_params: Vec<LifetimeParam>,
    fresh_lifetime_name: String,
    source_lifetime: SourceLifetime,
    type_params: Vec<(Ident, Option<Type>)>,
}

impl Default for TypeParams {
    fn default() -> Self {
        Self {
            lifetime_params: Default::default(),
            fresh_lifetime_name: String::from("'s"),
            source_lifetime: Default::default(),
            type_params: Default::default(),
        }
    }
}

impl TypeParams {
    pub fn add_lifetime(&mut self, lt: LifetimeParam) {
        self.lifetime_params.push(lt);
        while self
            .lifetime_params
            .iter()
            .any(|lt| lt.lifetime.ident == self.fresh_lifetime_name.trim_start_matches("\'"))
        {
            self.fresh_lifetime_name += "_";
        }
    }

    pub fn add_type(&mut self, param: Ident) {
        self.type_params.push((param, None));
    }

    pub fn set_type(&mut self, param: Ident, ty: TokenStream, errors: &mut Errors) {
        let ty = match syn::parse2::<Type>(ty) {
            Ok(mut ty) => {
                self.fix_source_lifetime_implicit(&mut ty);
                ty
            }
            Err(err) => {
                errors.err(err.to_string(), err.span());
                return;
            }
        };

        match self.type_params.iter_mut().find(|(name, _)| *name == param) {
            Some((_, slot)) => {
                if let Some(previous) = slot.replace(ty) {
                    errors
                        .err(
                            format!("{param} can only have one type assigned to it"),
                            param.span(),
                        )
                        .err("Previously assigned here", previous.span());
                }
            }
            None => {
                errors.err(
                    format!("{param} is not a declared type parameter"),
                    param.span(),
                );
            }
        }
    }

    pub fn set_source_lifetime(&mut self, source_lifetime: TokenStream, errors: &mut Errors) {
        mod kw {
            syn::custom_keyword!(none);
        }

        if let Ok(none) = syn::parse2::<kw::none>(source_lifetime.clone()) {
            self.source_lifetime = SourceLifetime::Fresh(none.span());
        } else {
            match syn::parse2::<Lifetime>(source_lifetime) {
                Ok(lt) => {
                    if self.lifetime_params.iter().all(|ltp| ltp.lifetime != lt) {
                        let list = self
                            .lifetime_params
                            .iter()
                            .map(|lt| format!("`{}`", lt.lifetime))
                            .collect::<Vec<_>>()
                            .join(", ");

                        errors.err(
                            format!("Lifetime `{lt}` not found in parameters\nAvailable lifetimes: {list}"),
                            lt.span(),
                        );
                    }
                    self.source_lifetime = SourceLifetime::Named(lt);
                }
                Err(err) => {
                    errors.err(err.to_string(), err.span());
                }
            }
        }
    }

    pub fn source_lifetime_span(&self) -> Option<Span> {
        match &self.source_lifetime {
            SourceLifetime::Implicit => None,
            SourceLifetime::Fresh(span) => Some(*span),
            SourceLifetime::Named(lifetime) => Some(lifetime.span()),
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

    pub fn source_lifetime(&self, errors: Option<&mut Errors>) -> TokenStream {
        match &self.source_lifetime {
            SourceLifetime::Implicit => {
                if let Some(errors) = errors {
                    if self.lifetime_params.len() > 1 {
                        self.lifetime_params.iter().fold(errors, |errors, lt| {
                                errors.err(
                                    format!("Source lifetime must be explicitly specified when more than one lifetime is present\n\
                                    Use #[logos(lifetime = {})] to use this lifetime for the source", lt.lifetime), lt.span())
                            });
                    }
                }
                let lt = Lifetime::new("'s", Span::call_site());
                quote!(#lt)
            }
            SourceLifetime::Fresh(_) => {
                let lt = Lifetime::new(&self.fresh_lifetime_name, Span::call_site());
                quote!(#lt)
            }
            SourceLifetime::Named(lt) => quote!(#lt),
        }
    }

    pub fn generics(&self, errors: &mut Errors) -> Option<TokenStream> {
        if self.lifetime_params.is_empty() && self.type_params.is_empty() {
            return None;
        }

        let mut generics = self
            .lifetime_params
            .iter()
            .map(|lt| {
                let lt = &lt.lifetime;
                quote!(#lt)
            })
            .collect::<Vec<_>>();

        // Rename first lifetime to 's when source lifetime is implicit for backwards compatibility
        if matches!(&self.source_lifetime, SourceLifetime::Implicit) {
            if let Some(lt) = generics.first_mut() {
                *lt = self.source_lifetime(None);
            }
        }

        for (ty, replace) in self.type_params.iter() {
            match replace {
                Some(ty) => generics.push(quote!(#ty)),
                None => {
                    errors.err(
                        format!(
                            "Generic type parameter without a concrete type\n\
                            \n\
                            Define a concrete type Logos can use: #[logos(type {ty} = Type)]",
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

    pub fn lifetime_bounds(&self) -> TokenStream {
        let mut bounds = match self.source_lifetime {
            SourceLifetime::Implicit | SourceLifetime::Named(_) => Vec::new(),
            SourceLifetime::Fresh(_) => vec![self.source_lifetime(None)],
        };

        bounds.extend(self.lifetime_params.iter().map(|lt| quote!(#lt)));

        if matches!(self.source_lifetime, SourceLifetime::Implicit) {
            let lt = Lifetime::new("'s", Span::call_site());
            if bounds.is_empty() {
                bounds.push(quote!(#lt));
            } else {
                bounds[0] = quote!(#lt);
            }
        }

        quote!(<#(#bounds),*>)
    }

    /// Replaces all lifetimes with 's when source lifetime is implicit for backwards compatibility
    pub fn fix_source_lifetime_implicit(&self, ty: &mut Type) {
        if matches!(&self.source_lifetime, SourceLifetime::Implicit) {
            replace_lifetimes(ty);
        }
    }
}

pub fn replace_lifetimes(ty: &mut Type) {
    traverse_type(ty, &mut replace_lifetime)
}

pub fn replace_lifetime(ty: &mut Type) {
    use syn::{GenericArgument, PathArguments};

    match ty {
        Type::Path(p) => {
            p.path
                .segments
                .iter_mut()
                .filter_map(|segment| match &mut segment.arguments {
                    PathArguments::AngleBracketed(ab) => Some(ab),
                    _ => None,
                })
                .flat_map(|ab| ab.args.iter_mut())
                .for_each(|arg| {
                    if let GenericArgument::Lifetime(lt) = arg {
                        *lt = Lifetime::new("'s", lt.span());
                    }
                });
        }
        Type::Reference(r) => {
            let span = match r.lifetime.take() {
                Some(lt) => lt.span(),
                None => Span::call_site(),
            };

            r.lifetime = Some(Lifetime::new("'s", span));
        }
        _ => (),
    }
}

pub fn traverse_type(ty: &mut Type, f: &mut impl FnMut(&mut Type)) {
    f(ty);
    match ty {
        Type::Array(array) => traverse_type(&mut array.elem, f),
        Type::BareFn(bare_fn) => {
            for input in &mut bare_fn.inputs {
                traverse_type(&mut input.ty, f);
            }
            if let syn::ReturnType::Type(_, ty) = &mut bare_fn.output {
                traverse_type(ty, f);
            }
        }
        Type::Group(group) => traverse_type(&mut group.elem, f),
        Type::Paren(paren) => traverse_type(&mut paren.elem, f),
        Type::Path(path) => traverse_path(&mut path.path, f),
        Type::Ptr(p) => traverse_type(&mut p.elem, f),
        Type::Reference(r) => traverse_type(&mut r.elem, f),
        Type::Slice(slice) => traverse_type(&mut slice.elem, f),
        Type::TraitObject(object) => object.bounds.iter_mut().for_each(|bound| {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                traverse_path(&mut trait_bound.path, f);
            }
        }),
        Type::Tuple(tuple) => tuple
            .elems
            .iter_mut()
            .for_each(|elem| traverse_type(elem, f)),
        _ => (),
    }
}

fn traverse_path(path: &mut Path, f: &mut impl FnMut(&mut Type)) {
    for segment in &mut path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => (),
            syn::PathArguments::AngleBracketed(args) => {
                for arg in &mut args.args {
                    match arg {
                        syn::GenericArgument::Type(ty) => {
                            traverse_type(ty, f);
                        }
                        syn::GenericArgument::AssocType(assoc) => {
                            traverse_type(&mut assoc.ty, f);
                        }
                        _ => (),
                    }
                }
            }
            syn::PathArguments::Parenthesized(args) => {
                for arg in &mut args.inputs {
                    traverse_type(arg, f);
                }
                if let syn::ReturnType::Type(_, ty) = &mut args.output {
                    traverse_type(ty, f);
                }
            }
        }
    }
}
