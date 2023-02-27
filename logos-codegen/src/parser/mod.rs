use beef::lean::Cow;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, GenericParam, Type};

use crate::error::Errors;
use crate::leaf::{Callback, InlineCallback};
use crate::parse::prelude::*;
use crate::util::MaybeVoid;
use crate::LOGOS_ATTR;

mod definition;
mod ignore_flags;
mod nested;
mod subpattern;
mod type_params;

pub use self::definition::{Definition, Literal};
pub use self::ignore_flags::IgnoreFlags;
pub use self::subpattern::Subpatterns;
use self::type_params::{replace_lifetime, traverse_type, TypeParams};

use nested::{NestedAssign, NestedKeywordAssign, Splitter};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    pub mode: Mode,
    pub source: Option<TokenStream>,
    pub skips: Vec<Literal>,
    pub extras: MaybeVoid,
    pub error_type: MaybeVoid,
    pub subpatterns: Subpatterns,
    pub logos_path: Option<TokenStream>,
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

    fn parse_attr(&mut self, attr: &mut Attribute) -> Option<Splitter> {
        let mut tokens = std::mem::replace(&mut attr.tokens, TokenStream::new()).into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => Some(Splitter::new(group.stream())),
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
            Some(nested) => nested,
            None => {
                self.err("Expected: #[logos(...)]", attr.span());
                return;
            }
        };

        for stream in nested {
            let mut stream = stream.parse_stream();

            let name: Ident = match stream.parse() {
                Ok(ident) => ident,
                Err(err) => {
                    self.errors.push(err);
                    continue;
                }
            };

            type Handler =
                fn(&mut Parser, Span, stream: &mut ParseStream) -> Result<(), ParseError>;

            // IMPORTANT: Keep these sorted alphabetically for binary search down the line
            static NESTED_ATTRS: &[(&str, &str, Handler)] = &[
                (
                    "crate",
                    "Expected: #[logos(crate = path::to::logos)]",
                    |parser, span, stream| {
                        let NestedAssign { value } = stream.parse::<NestedAssign>()?;

                        if let Some(previous) = parser.logos_path.replace(value) {
                            parser
                                .err("#[logos(crate)] can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }

                        Ok(())
                    },
                ),
                (
                    "error",
                    "Expected: #[logos(error = SomeType)]",
                    |parser, span, stream| {
                        let NestedAssign { value } = stream.parse::<NestedAssign>()?;

                        if let MaybeVoid::Some(previous) = parser.error_type.replace(value) {
                            parser
                                .err("#[logos(error)] can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }

                        Ok(())
                    },
                ),
                (
                    "extras",
                    "Expected: #[logos(extras = SomeType)]",
                    |parser, span, stream| {
                        let NestedAssign { value } = stream.parse::<NestedAssign>()?;

                        if let MaybeVoid::Some(previous) = parser.extras.replace(value) {
                            parser
                                .err("#[logos(extras)] can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }

                        Ok(())
                    },
                ),
                (
                    "skip",
                    "Expected: #[logos(skip \"regex literal\")]",
                    |parser, _, stream| {
                        parser.skips.push(stream.parse()?);

                        Ok(())
                    },
                ),
                (
                    "source",
                    "Expected: #[logos(source = SomeType)]",
                    |parser, span, stream| {
                        let NestedAssign { value } = stream.parse::<NestedAssign>()?;

                        if let Some(previous) = parser.source.replace(value) {
                            parser
                                .err("#[logos(source)] can be defined only once", span)
                                .err("Previous definition here", previous.span());
                        }

                        Ok(())
                    },
                ),
                (
                    "subpattern",
                    "Expected: #[logos(subpattern name = r\"regex\")]",
                    |parser, _, stream| {
                        let NestedKeywordAssign { name, value } = stream.parse()?;

                        parser.subpatterns.add(name, value, &mut parser.errors);

                        Ok(())
                    },
                ),
                (
                    "type",
                    "Expected: #[logos(type T = SomeType)]",
                    |parser, _, stream| {
                        let NestedKeywordAssign { name, value } = stream.parse()?;

                        parser.types.set(name, value, &mut parser.errors);

                        Ok(())
                    },
                ),
            ];

            match name.with_str(|name| NESTED_ATTRS.binary_search_by_key(&name, |(n, _, _)| n)) {
                Ok(idx) => {
                    let span = name.span();
                    let (_, expected, handler) = NESTED_ATTRS[idx];
                    if let Err(err) = (handler)(self, span, &mut stream) {
                        self.err(expected, span).push(err);
                    }
                }
                Err(_) => {
                    let mut err = format!(
                        "Unknown nested attribute #[logos({name})], expected one of: {}",
                        NESTED_ATTRS[0].0
                    );

                    for (allowed, _, _) in &NESTED_ATTRS[1..] {
                        err.push_str(", ");
                        err.push_str(allowed);
                    }

                    self.err(err, name.span());
                }
            }
        }
    }

    /// Parse attribute definition of a token:
    ///
    /// + `#[token(literal[, callback])]`
    /// + `#[regex(literal[, callback])]`
    pub fn parse_definition(&mut self, attr: &mut Attribute) -> Option<Definition> {
        let span = attr.tokens.span();
        let mut nested = self.parse_attr(attr)?;

        let mut stream = match nested.next() {
            Some(tokens) => tokens.parse_stream(),
            None => {
                self.err("Expected a literal", span);

                return None;
            }
        };

        let literal = match stream.parse() {
            Ok(lit) => lit,
            Err(err) => {
                self.errors.push(err);

                return None;
            }
        };

        if matches!(literal, Literal::Bytes(_)) {
            self.mode = Mode::Binary;
        }

        let mut def = Definition::new(literal);

        for stream in nested {
            let mut stream = stream.parse_stream();

            match stream.peek() {
                Some(TokenTree::Ident(_)) => (),
                _ => match stream.parse::<Callback>() {
                    Ok(callback) => {
                        def.callback = Some(callback);
                    }
                    Err(err) => {
                        self.errors.push(err);
                    }
                },
            }

            if let Some(tokens) = def.named_attr(&mut stream, &mut self.errors) {
                def.callback = Some(Callback::Label(tokens));
            }
        }

        Some(def)
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
                    if let Some(ident) = tp.path.get_ident() {
                        if let Some(substitue) = self.types.find(&ident.to_string()) {
                            *ty = substitue;
                        }
                    }
                }
            }
            // If `ty` is a concrete type, fix its lifetimes to 'source
            replace_lifetime(ty);
        });

        quote!(#ty)
    }

    pub fn err<M, S>(&mut self, message: M, span: S) -> &mut Errors
    where
        M: Into<Cow<'static, str>>,
        S: IntoSpan,
    {
        self.errors.err(message, span)
    }
}

impl Parse for Callback {
    fn parse(stream: &mut ParseStream) -> Result<Self, ParseError> {
        if stream.allow_consume('|').is_some() {
            let arg = stream.parse::<Ident>()?;
            let span = arg.span();

            stream.expect('|').map_err(|err| {
                err.explain("Inline callbacks must use closure syntax with exactly one parameter")
            })?;

            let body = match stream.allow_consume('{') {
                Some(TokenTree::Group(group)) => group.stream(),
                _ => stream.collect(),
            };

            let arg = proc_macro2::Ident::new(&arg.to_string(), arg.span());

            return Ok(InlineCallback { arg, body, span }.into());
        }

        Ok(Callback::Label(stream.collect()))
    }
}
