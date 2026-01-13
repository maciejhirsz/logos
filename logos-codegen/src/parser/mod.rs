use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use std::borrow::Cow;
use syn::spanned::Spanned;
use syn::{Attribute, GenericParam, Ident, Lit, LitBool, Meta, Type};

use crate::error::Errors;
use crate::leaf::{Callback, InlineCallback};
use crate::util::{expect_punct, MaybeVoid};
use crate::LOGOS_ATTR;

mod definition;
mod error_type;
mod ignore_flags;
mod nested;
mod subpattern;
mod type_params;

pub use self::definition::{Definition, Literal};
pub use self::error_type::ErrorType;
pub use self::ignore_flags::IgnoreFlags;
use self::nested::{AttributeParser, Nested, NestedValue};
pub use self::subpattern::Subpatterns;
use self::type_params::{replace_lifetime, traverse_type, TypeParams};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    pub utf8_mode: Option<LitBool>,
    pub skips: Vec<Definition>,
    pub extras: MaybeVoid,
    pub subpatterns: Vec<(Ident, Literal)>,
    pub error_type: Option<ErrorType>,
    pub logos_path: Option<TokenStream>,
    pub export_path: Option<String>,
    types: TypeParams,
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
        match &mut attr.meta {
            Meta::List(list) => {
                let tokens = std::mem::replace(&mut list.tokens, TokenStream::new());

                Some(AttributeParser::new(tokens))
            }
            _ => None,
        }
    }

    /// Try to parse the main `#[logos(...)]`, does nothing if
    /// the attribute's name isn't `logos`.
    pub fn try_parse_logos(&mut self, attr: &mut Attribute) {
        if !attr.path().is_ident(LOGOS_ATTR) {
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

            let span = name.span();

            match name.to_string().as_str() {
                "crate" => match value {
                    NestedValue::Assign(logos_path) => self.logos_path = Some(logos_path),
                    _ => {
                        self.err("Expected: #[logos(crate = path::to::logos)]", span);
                    }
                },
                "error" => match value {
                    NestedValue::Assign(value) => {
                        let span = value.span();

                        let error_ty = ErrorType::new(value);

                        if let Some(previous) = self.error_type.replace(error_ty) {
                            self.err("Error type can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }
                    }
                    NestedValue::Group(value) => {
                        let span = value.span();
                        let mut nested = AttributeParser::new(value);
                        let ty = match nested.parsed::<Type>() {
                            Some(Ok(ty)) => ty,
                            Some(Err(e)) => {
                                self.err(e.to_string(), e.span());
                                return;
                            }
                            None => {
                                self.err("Expected #[logos(error(SomeType))]", span);
                                return;
                            }
                        };

                        let mut error_type = {
                            use quote::ToTokens;
                            ErrorType::new(ty.into_token_stream())
                        };

                        for (position, next) in nested.enumerate() {
                            match next {
                                Nested::Unexpected(tokens) => {
                                    self.err("Unexpected token in attribute", tokens.span());
                                }
                                Nested::Unnamed(tokens) => match position {
                                    0 => error_type.callback = self.parse_callback(tokens),
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
                                    error_type.named_attr(name, value, self);
                                }
                            }
                        }

                        if let Some(previous) = self.error_type.replace(error_type) {
                            self.err("Error type can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }
                    }
                    _ => {
                        self.err(
                            concat!(
                                "Expected: #[logos(error = SomeType)] or ",
                                "#[logos(error(SomeType[, callback))]"
                            ),
                            span,
                        );
                    }
                },
                "export_dir" => match value {
                    NestedValue::Assign(value) => {
                        let span = value.span();

                        match syn::parse2::<Literal>(value) {
                            Ok(Literal::Utf8(str)) => {
                                if let Some(previous) = self.export_path.replace(str.value()) {
                                    self.err("Export path can be defined only once", span)
                                        .err("Previous definition here", previous.span());
                                }
                            }
                            Ok(_) => {
                                self.err("Expected a &str", span);
                            }
                            Err(e) => {
                                self.err(e.to_string(), span);
                            }
                        }
                    }
                    _ => {
                        self.err(
                            "Expected #[logos(export_dir = \"path/to/export/dir\")]",
                            span,
                        );
                    }
                },
                "extras" => match value {
                    NestedValue::Assign(value) => {
                        let span = value.span();

                        if let MaybeVoid::Some(previous) = self.extras.replace(value) {
                            self.err("Extras can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }
                    }
                    _ => {
                        self.err("Expected: #[logos(extras = SomeType)]", span);
                    }
                },
                "skip" => match value {
                    NestedValue::Literal(lit) => {
                        if let Some(literal) = self.parse_literal(Lit::new(lit)) {
                            self.skips.push(Definition::new(literal));
                        }
                    }
                    NestedValue::Group(tokens) => {
                        let token_span = tokens.span();
                        if let Some(skip) = self.parse_definition(AttributeParser::new(tokens)) {
                            self.skips.push(skip);
                        } else {
                            self.err(
                                "Expected #[logos(skip(\"regex literal\"[, [callback = ] callback, priority = priority]))]",
                                token_span,
                            );
                        }
                    }
                    _ => {
                        self.err(
                            "Expected: #[logos(skip \"regex literal\")] or #[logos(skip(...))]",
                            span,
                        );
                    }
                },
                "source" => {
                    self.err(
                        "The `source` attribute is deprecated. Use the `utf8` attribute instead",
                        span,
                    );
                }
                "subpattern" => match value {
                    NestedValue::KeywordAssign(name, value) => {
                        match syn::parse2::<Literal>(value) {
                            Ok(lit) => {
                                self.subpatterns.push((name, lit));
                            }
                            Err(e) => {
                                self.errors.err(e.to_string(), e.span());
                            }
                        };
                    }
                    _ => {
                        self.err(r#"Expected: #[logos(subpattern name = r"regex")]"#, span);
                    }
                },
                "type" => match value {
                    NestedValue::KeywordAssign(generic, ty) => {
                        self.types.set(generic, ty, &mut self.errors);
                    }
                    _ => {
                        self.err("Expected: #[logos(type T = SomeType)]", span);
                    }
                },
                "utf8" => match value {
                    NestedValue::Assign(value) => {
                        let span = value.span();

                        match syn::parse2::<LitBool>(value) {
                            Ok(lit) => {
                                if let Some(previous) = self.utf8_mode.replace(lit) {
                                    self.err("Utf8 mode can be defined only once", span)
                                        .err("Previous definition here", previous.span());
                                }
                            }
                            Err(e) => {
                                self.err(format!("Expected a boolean literal: {e}"), span);
                            }
                        }
                    }
                    _ => {
                        self.err("Expected: #[logos(utf8 = true)]", span);
                    }
                },
                name => {
                    self.err(format!("Unknown nested attribute #[logos({name})]",), span);
                }
            }
        }
    }

    pub fn parse_literal(&mut self, lit: Lit) -> Option<Literal> {
        match lit {
            Lit::Str(string) => Some(Literal::Utf8(string)),
            Lit::ByteStr(bytes) => Some(Literal::Bytes(bytes)),
            _ => {
                self.err("Expected a &str or &[u8] slice", lit.span());

                None
            }
        }
    }

    /// Parse attribute definition of a token:
    ///
    /// + `#[token(literal[, callback])]`
    /// + `#[regex(literal[, callback])]`
    pub fn parse_definition_attr(&mut self, attr: &mut Attribute) -> Option<Definition> {
        let nested = self.parse_attr(attr)?;

        self.parse_definition(nested)
    }

    fn parse_definition(&mut self, mut nested: AttributeParser) -> Option<Definition> {
        let literal = match nested.parsed::<Lit>()? {
            Ok(lit) => self.parse_literal(lit)?,
            Err(err) => {
                self.err(err.to_string(), err.span());

                return None;
            }
        };

        let mut skip = Definition::new(literal);

        for (position, next) in nested.enumerate() {
            match next {
                Nested::Unexpected(tokens) => {
                    self.err("Unexpected token in attribute", tokens.span());
                }
                Nested::Unnamed(tokens) => match position {
                    0 => skip.callback = self.parse_callback(tokens),
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
                    skip.named_attr(name, value, self);
                }
            }
        }

        Some(skip)
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
                    if let Some(substitute) = self.types.find(&tp.path) {
                        *ty = substitute;
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
