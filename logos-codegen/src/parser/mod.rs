use beef::lean::Cow;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, GenericParam, Lit, Type};

use crate::error::Errors;
use crate::leaf::{Callback, InlineCallback};
use crate::parse::prelude::*;
use crate::parse::TokenStreamExt;
use crate::util::{expect_punct, MaybeVoid};
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

use nested::{AttributeParser, CommaSplitter, Nested, NestedAssign, NestedKeywordAssign};

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
                self.types.add2(ty.ident);
            }
            GenericParam::Const(c) => {
                self.err("Logos doesn't support const generics.", c.span());
            }
        }
    }

    pub fn generics(&mut self) -> Option<TokenStream> {
        self.types.generics(&mut self.errors)
    }

    fn parse_attr2(&mut self, attr: &mut Attribute) -> Option<AttributeParser> {
        let mut tokens = std::mem::replace(&mut attr.tokens, TokenStream::new()).into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => Some(AttributeParser::new(group.stream())),
            _ => None,
        }
    }

    fn parse_attr(&mut self, attr: &mut Attribute) -> Option<CommaSplitter> {
        let mut tokens = std::mem::replace(&mut attr.tokens, TokenStream::new()).into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => Some(CommaSplitter::new(group.stream())),
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

            let name: proc_macro::Ident = match stream.parse() {
                Ok(ident) => ident,
                Err(err) => {
                    self.errors.push(err);
                    continue;
                }
            };

            type Callback = fn(
                &mut Parser,
                proc_macro::Span,
                stream: &mut ParseStream,
            ) -> Result<(), ParseError>;

            // IMPORTANT: Keep these sorted alphabetically for binary search down the line
            static NESTED_LOOKUP: &[(&str, &str, Callback)] = &[
                (
                    "crate",
                    "Expected: #[logos(crate = path::to::logos)]",
                    |parser, span, stream| {
                        let NestedAssign { value } = stream.parse::<NestedAssign>()?;

                        if let Some(previous) = parser.logos_path.replace(value.into()) {
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

                        if let MaybeVoid::Some(previous) = parser.error_type.replace(value.into()) {
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

                        if let MaybeVoid::Some(previous) = parser.extras.replace(value.into()) {
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

                        if let Some(previous) = parser.source.replace(value.into()) {
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
                    }
                ),
                (
                    "type",
                    "Expected: #[logos(type T = SomeType)]",
                    |parser, _, stream| {
                        let NestedKeywordAssign { name, value } = stream.parse()?;

                        parser.types.set(name, value, &mut parser.errors);

                        Ok(())
                    }
                )
            ];

            name.with_str(
                |nstr| match NESTED_LOOKUP.binary_search_by_key(&nstr, |(n, _, _)| n) {
                    Ok(idx) => {
                        let span = name.span();
                        let (_, expected, callback) = NESTED_LOOKUP[idx];
                        if let Err(err) = (callback)(self, span, &mut stream) {
                            self.err(expected, span).push(err);
                        }
                    }
                    Err(_) => {
                        let mut err = format!(
                            "Unknown nested attribute #[logos({name})], expected one of: {}",
                            NESTED_LOOKUP[0].0
                        );

                        for (allowed, _, _) in &NESTED_LOOKUP[1..] {
                            err.push_str(", ");
                            err.push_str(allowed);
                        }

                        self.err(err, name.span());
                    }
                },
            );
        }
    }

    pub fn parse_literal(&mut self, lit: Lit) -> Option<Literal> {
        match lit {
            Lit::Str(string) => Some(Literal::Utf8(string)),
            Lit::ByteStr(bytes) => {
                self.mode = Mode::Binary;

                Some(Literal::Bytes(bytes))
            }
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
    pub fn parse_definition(&mut self, attr: &mut Attribute) -> Option<Definition> {
        let mut nested = self.parse_attr2(attr)?;

        let literal = match nested.parsed::<Lit>()? {
            Ok(lit) => self.parse_literal(lit)?,
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
