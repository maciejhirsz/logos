use beef::lean::Cow;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, GenericParam, Type, spanned::Spanned};

use crate::util;
use crate::error::Errors;
use crate::parsers::{AttributeParser, Nested, NestedValue};
use crate::type_params::{TypeParams, replace_lifetimes};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    extras: Option<TokenStream>,
    types: TypeParams,
}

impl Parser {
    pub fn extras(&mut self) -> TokenStream {
        self.extras.take().unwrap_or_else(|| quote!(()))
    }

    pub fn parse_generic(&mut self, param: GenericParam) {
        match param {
            GenericParam::Lifetime(lt) => {
                self.types.explicit_lifetime(lt, &mut self.errors);
            },
            GenericParam::Type(ty) => {
                self.types.add(ty.ident);
            },
            GenericParam::Const(c) => {
                self.err(
                    "Logos doesn't support const generics.",
                    c.span(),
                );
            },
        }
    }

    pub fn generics(&mut self) -> Option<TokenStream> {
        self.types.generics(&mut self.errors)
    }

    pub fn try_parse_logos(&mut self, attr: &Attribute) {
        if !attr.path.is_ident("logos") {
            return;
        }

        let nested = util::read_attr("logos", attr, &mut self.errors);

        for nested in AttributeParser::new(nested) {
            let (name, value) = match nested {
                Nested::Named(name, value) => (name, value),
                Nested::Unexpected(unexpected) => {
                    self.err("Unexpected tokens in attribute", unexpected.span());
                    continue;
                },
                Nested::Unnamed(unnamed) => {
                    self.err("Expected a named nested attribute", unnamed.span());
                    continue;
                },
            };

            match (name.to_string().as_str(), value) {
                ("extras", NestedValue::Assign(value)) => {
                    let span = value.span();

                    if let Some(previous) = self.extras.replace(value) {
                        self.err("Extras can be defined only once", span)
                            .err("Previous definition here", previous.span());
                    }
                },
                ("extras", _) => {
                    self.err("Expected: extras = SomeType", name.span());
                }
                ("type", NestedValue::KeywordAssign(generic, ty)) => {
                    self.types.set(generic, ty, &mut self.errors);
                },
                ("type", _) => {
                    self.err("Expected: type T = SomeType", name.span());
                },
                ("trivia", _) => {
                    // TODO: Remove in future versions
                    self.err(
                        "\
                        trivia are no longer supported.\n\n\

                        For help with migration see release notes: \
                        https://github.com/maciejhirsz/logos/releases\
                        ",
                        name.span(),
                    );
                },
                (unknown, _) => {
                    self.err(
                        format!("Unknown nested attribute: {}", unknown),
                        name.span(),
                    );
                }
            }
        }
    }

    /// Checks if `ty` is a declared generic param, if so replaces it
    /// with a concrete type defined using #[logos(type T = Type)]
    ///
    /// If no matching generic param is found, all lifetimes are fixed
    /// to the source lifetime
    pub fn get_type(&self, mut ty: Type) -> Type {
        if let Type::Path(tp) = &ty {
            if tp.qself.is_some() {
                return ty;
            }

            if let Some(substitue) = self.types.find(&tp.path) {
                return substitue;
            }
        }

        replace_lifetimes(&mut ty);
        ty
    }

    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Errors
    where
        M: Into<Cow<'static, str>>,
    {
        self.errors.err(message, span)
    }
}
