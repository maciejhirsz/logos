use beef::lean::Cow;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, GenericParam, Lit, Path, Type};

use crate::error::Errors;
use crate::leaf::{Callback, InlineCallback};
use crate::util::{expect_punct, MaybeVoid};
use crate::LOGOS_ATTR;

mod definition;
mod ignore_flags;
mod nested;
mod subpattern;
mod type_params;

pub use self::definition::{Definition, Literal};
pub use self::ignore_flags::IgnoreFlags;
use self::nested::{AttributeParser, Nested, NestedValue};
pub use self::subpattern::Subpatterns;
use self::type_params::{replace_lifetime, traverse_type, TypeParams};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    pub mode: Mode,
    pub source: Option<TokenStream>,
    pub extras: MaybeVoid,
    pub error_type: MaybeVoid,
    pub subpatterns: Subpatterns,
    pub logos_path: Option<Path>,
    types: TypeParams,
}

pub enum Mode {
    Utf8,
    Binary,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Utf8
    }
}

impl Parser {
    pub fn parse_generic(&mut self, param: GenericParam) {
        match param {
            GenericParam::Lifetime(lt) => {
                self.types.explicit_lifetime(lt, &mut self.errors);
            }
            GenericParam::Type(ty) => {
                self.types.add(ty.ident);
            }
            GenericParam::Const(c) => {
                self.err("Logos doesn't support const generics.", c.span());
            }
        }
    }

    pub fn generics(&mut self) -> Option<TokenStream> {
        self.types.generics(&mut self.errors)
    }

    fn parse_attr(&mut self, attr: &mut Attribute) -> Option<AttributeParser> {
        let mut tokens = std::mem::replace(&mut attr.tokens, TokenStream::new()).into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => Some(AttributeParser::new(group.stream())),
            _ => None,
        }
    }

    /// Try to parse the main `#[logos(...)]`, does nothing if
    /// the attribute's name isn't `logos`.
    pub fn try_parse_logos(&mut self, attr: &mut Attribute) {
        if !attr.path.is_ident(LOGOS_ATTR) {
            return;
        }

        let nested = match self.parse_attr(attr) {
            Some(tokens) => tokens,
            None => {
                self.err("Expected #[logos(...)]", attr.span());
                return;
            }
        };

        for nested in nested {
            let (name, value) = match nested {
                Nested::Named(name, value) => (name, value),
                Nested::Unexpected(tokens) | Nested::Unnamed(tokens) => {
                    self.err("Invalid nested attribute", tokens.span());
                    continue;
                }
            };

            match (name.to_string().as_str(), value) {
                ("skip", NestedValue::Literal(lit)) => {}
                ("skip", _) => {
                    self.err("Expected: #[logos(skip \"regex literal\")]", name.span());
                }
                ("source", NestedValue::Assign(value)) => {
                    let span = value.span();
                    if let Some(previous) = self.source.replace(value) {
                        self.err("Source can be defined only once", span)
                            .err("Previous definition here", previous.span());
                    }
                }
                ("source", _) => {
                    self.err("Expected: #[logos(source = SomeType)]", name.span());
                }
                ("error", NestedValue::Assign(value)) => {
                    let span = value.span();

                    if let MaybeVoid::Some(previous) = self.error_type.replace(value) {
                        self.err("Error type can be defined only once", span)
                            .err("Previous definition here", previous.span());
                    }
                }
                ("error", _) => {
                    self.err("Expected: #[logos(error = SomeType)]", name.span());
                }
                ("extras", NestedValue::Assign(value)) => {
                    let span = value.span();

                    if let MaybeVoid::Some(previous) = self.extras.replace(value) {
                        self.err("Extras can be defined only once", span)
                            .err("Previous definition here", previous.span());
                    }
                }
                ("extras", _) => {
                    self.err("Expected: #[logos(extras = SomeType)]", name.span());
                }
                ("type", NestedValue::KeywordAssign(generic, ty)) => {
                    self.types.set(generic, ty, &mut self.errors);
                }
                ("type", _) => {
                    self.err("Expected: #[logos(type T = SomeType)]", name.span());
                }
                ("subpattern", NestedValue::KeywordAssign(name, value)) => {
                    self.subpatterns.add(name, value, &mut self.errors);
                }
                ("subpattern", _) => {
                    self.err(
                        r#"Expected: #[logos(subpattern name = r"regex")]"#,
                        name.span(),
                    );
                }
                ("crate", NestedValue::Assign(value)) => match syn::parse2(value) {
                    Ok(logos_path) => self.logos_path = Some(logos_path),
                    Err(_) => {
                        self.err(
                            r#"Expected: #[logos(crate = some::path::to::logos)]"#,
                            name.span(),
                        );
                    }
                },
                ("crate", _) => {
                    self.err(
                        r#"Expected: #[logos(crate = some::path::to::logos)]"#,
                        name.span(),
                    );
                }
                (unknown, _) => {
                    self.err(
                        format!("Unknown nested attribute: {}", unknown),
                        name.span(),
                    );
                }
            }
        }
    }

    /// Parse attribute definition of a token:
    ///
    /// + `#[token(literal[, callback])]`
    /// + `#[regex(literal[, callback])]`
    pub fn parse_definition(&mut self, attr: &mut Attribute) -> Option<Definition> {
        let mut nested = self.parse_attr(attr)?;

        let literal = match nested.parsed::<Lit>()? {
            Ok(lit) => match lit {
                Lit::Str(string) => Literal::Utf8(string),
                Lit::ByteStr(bytes) => {
                    self.mode = Mode::Binary;

                    Literal::Bytes(bytes)
                }
                _ => {
                    self.err("Expected a &str or &[u8] slice", lit.span());

                    return None;
                }
            },
            Err(err) => {
                self.err(err.to_string(), err.span());

                return None;
            }
        };

        let mut def = Definition::new(literal);

        for (position, next) in nested.enumerate() {
            match next {
                Nested::Unexpected(tokens) => {
                    self.err("Unexpected token in attribute", tokens.span());
                }
                Nested::Unnamed(tokens) => match position {
                    0 => def.callback = self.parse_callback(tokens),
                    _ => {
                        self.err(
                            "\
                            Expected a named argument at this position\n\
                            \n\
                            hint: If you are trying to define a callback here use: callback = ...\
                            ",
                            tokens.span(),
                        );
                    }
                },
                Nested::Named(name, value) => {
                    def.named_attr(name, value, self);
                }
            }
        }

        Some(def)
    }

    fn parse_callback(&mut self, tokens: TokenStream) -> Option<Callback> {
        let span = tokens.span();
        let mut tokens = tokens.into_iter();

        if let Some(tt) = expect_punct(tokens.next(), '|') {
            let mut label = TokenStream::from(tt);

            label.extend(tokens);

            return Some(Callback::Label(label));
        }

        let first = tokens.next();
        let error = expect_punct(tokens.next(), '|');

        let arg = match (error, first) {
            (None, Some(TokenTree::Ident(arg))) => arg,
            _ => {
                self.err(
                    "Inline callbacks must use closure syntax with exactly one parameter",
                    span,
                );
                return None;
            }
        };

        let body = match tokens.next() {
            Some(TokenTree::Group(group)) => group.stream(),
            Some(first) => {
                let mut body = TokenStream::from(first);

                body.extend(tokens);
                body
            }
            None => {
                self.err("Callback missing a body", span);
                return None;
            }
        };

        let inline = InlineCallback { arg, body, span };

        Some(inline.into())
    }

    /// Checks if `ty` is a declared generic param, if so replaces it
    /// with a concrete type defined using #[logos(type T = Type)]
    ///
    /// If no matching generic param is found, all lifetimes are fixed
    /// to the source lifetime
    pub fn get_type(&self, ty: &mut Type) -> TokenStream {
        traverse_type(ty, &mut |ty| {
            if let Type::Path(tp) = ty {
                // Skip types that begin with `self::`
                if tp.qself.is_none() {
                    // If `ty` is a generic type parameter, try to find
                    // its concrete type defined with #[logos(type T = Type)]
                    if let Some(substitue) = self.types.find(&tp.path) {
                        *ty = substitue;
                    }
                }
            }
            // If `ty` is a concrete type, fix its lifetimes to 'source
            replace_lifetime(ty);
        });

        quote!(#ty)
    }

    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Errors
    where
        M: Into<Cow<'static, str>>,
    {
        self.errors.err(message, span)
    }
}
