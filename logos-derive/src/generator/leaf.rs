use proc_macro2::{TokenStream, Span};
use syn::{Lifetime, Type};
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
            Some(mut ty) => {
                replace_lifetimes(&mut ty);

                (quote!(#ty), quote!(#name::#ident))
            },
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

fn replace_lifetimes(ty: &mut Type) {
    use syn::{PathArguments, GenericArgument};
    use syn::spanned::Spanned;

    match ty {
        Type::Array(array) => replace_lifetimes(&mut array.elem),
        Type::Group(group) => replace_lifetimes(&mut group.elem),
        Type::Paren(paren) => replace_lifetimes(&mut paren.elem),
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
                        GenericArgument::Type(ty) => {
                            replace_lifetimes(ty);
                        },
                        GenericArgument::Binding(bind) => {
                            replace_lifetimes(&mut bind.ty);
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
        Type::Slice(slice) => replace_lifetimes(&mut slice.elem),
        Type::Tuple(tuple) => tuple.elems.iter_mut().for_each(replace_lifetimes),
        _ => (),
    }
}
