use beef::lean::Cow;
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{Lit, Attribute, GenericParam, Type, spanned::Spanned};
use quote::quote;

use crate::error::Errors;
use crate::attr_parser::{AttributeParser, Nested, NestedValue};
use crate::type_params::{TypeParams, replace_lifetimes};
use crate::leaf::Callback;
use crate::util::{is_punct, Definition, Literal};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    types: TypeParams,
    extras: Option<TokenStream>,
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

    pub fn parse_attr(&mut self, name: &str, attr: &Attribute) -> TokenStream {
        let mut tokens = attr.tokens.clone().into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => group.stream(),
            _ => {
                self.err(format!("Expected #[{}(...)]", name), attr.span());

                TokenStream::new()
            }
        }
    }

    pub fn try_parse_logos(&mut self, attr: &Attribute) {
        if !attr.path.is_ident("logos") {
            return;
        }

        let nested = self.parse_attr("logos", attr);

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


    pub fn parse_definition(&mut self, name: &str, attr: &Attribute) -> Option<Definition> {
        let nested = self.parse_attr(name, attr);

        let mut nested = AttributeParser::new(nested);

        let literal = match nested.parsed::<Lit>()? {
            Ok(lit) => match lit {
                Lit::Str(ref v) => Literal::Utf8(v.value(), v.span()),
                Lit::ByteStr(ref v) => Literal::Bytes(v.value(), v.span()),
                _ => {
                    self.err("Expected a &str or &[u8] slice", lit.span());

                    return None;
                }
            },
            Err(err) => {
                self.err(err.to_string(), err.span());

                return None;
            },
        };

        let mut callback = Callback::None;

        if let Some(next) = nested.next() {
            match next {
                Nested::Unnamed(tokens) => {
                    callback = self.parse_callback(tokens);
                },
                Nested::Named(name, value) => {
                    match (name.to_string().as_str(), value) {
                        // ("priority", _) => {

                        // },
                        (unknown, _) => {
                            self.err(
                                format!("Unknown nested attribute: {}", unknown),
                                name.span(),
                            );
                        }
                    }
                },
                Nested::Unexpected(tokens) => {
                    self.err("Unexpected token in attribute", tokens.span());
                }
            };
        }

        Some(Definition {
            literal,
            callback,
        })
    }

    fn parse_callback(&mut self, tokens: TokenStream) -> Callback {
        let mut tokens = tokens.into_iter();

        let mut span = match tokens.next().unwrap() {
            tt if is_punct(&tt, '|') => tt.span(),
            tt => {
                let mut label = TokenStream::from(tt);

                label.extend(tokens);

                return Callback::Label(label);
            }
        };

        let ident = match tokens.next() {
            Some(TokenTree::Ident(ident)) => ident,
            _ => {
                self.err("Expected identifier following this token", span);
                return Callback::None;
            }
        };

        match tokens.next() {
            Some(tt) if is_punct(&tt, '|') => {
                span = span.join(tt.span()).unwrap();
            }
            _ => {
                self.err("Expected | following this token", ident.span());
                return Callback::None;
            }
        }

        let body = match tokens.next() {
            Some(TokenTree::Group(group)) => group.stream(),
            Some(first) => {
                let mut body = TokenStream::from(first);

                body.extend(tokens);
                body
            },
            None => {
                self.err("Callback missing a body", span);
                return Callback::None;
            }
        };

        Callback::Inline(ident, body)
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
