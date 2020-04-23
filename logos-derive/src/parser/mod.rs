use beef::lean::Cow;
use proc_macro2::{Ident, TokenStream, TokenTree, Span};
use syn::{Lit, LitStr, LitByteStr, Attribute, GenericParam, Type};
use syn::spanned::Spanned;
use quote::quote;

use crate::error::Errors;
use crate::leaf::{Callback, InlineCallback};
use crate::util::{expect_punct, bytes_to_regex_string, MaybeVoid};

mod nested;
mod type_params;

use self::nested::{AttributeParser, Nested, NestedValue};
use self::type_params::{TypeParams, replace_lifetimes};

#[derive(Default)]
pub struct Parser {
    pub errors: Errors,
    pub mode: Mode,
    pub extras: MaybeVoid,
    types: TypeParams,
}

pub struct Definition {
    pub literal: Literal,
    pub priority: Option<usize>,
    pub callback: Option<Callback>,
}

pub enum Mode {
    Utf8,
    Binary,
}

pub enum Literal {
    Utf8(LitStr),
    Bytes(LitByteStr),
}

impl Definition {
    fn new(literal: Literal) -> Self {
        Definition {
            literal,
            priority: None,
            callback: None,
        }
    }

    fn named_attr(&mut self, name: Ident, value: NestedValue, parser: &mut Parser) {
        match (name.to_string().as_str(), value) {
            ("priority", _) => {

            },
            ("callback", NestedValue::Assign(tokens)) => {
                let span = tokens.span();
                let callback = match parser.parse_callback(tokens) {
                    Some(callback) => callback,
                    None => {
                        parser.err("Not a valid callback", span);
                        return;
                    }
                };

                if let Some(previous) = self.callback.replace(callback) {
                    parser
                        .err(
                            "Callback has been already set",
                            span.join(name.span()).unwrap(),
                        )
                        .err("Previous callback set here", previous.span());
                }
            },
            ("callback", _) => {
                parser.err("Expected: callback = ...", name.span());
            },
            (unknown, _) => {
                parser.err(
                    format!(
                        "\
                        Unknown nested attribute: {}\n\n\

                        Expected one of: priority, callback\
                        ",
                        unknown
                    ),
                    name.span(),
                );
            }
        }
    }
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Utf8
    }
}

impl Literal {
    pub fn is_utf8(&self) -> bool {
        match self {
            Literal::Utf8(_) => true,
            Literal::Bytes(_) => false,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Literal::Utf8(string) => string.value().into_bytes(),
            Literal::Bytes(bytes) => bytes.value(),
        }
    }

    pub fn to_regex_string(&self) -> String {
        match self {
            Literal::Utf8(string) => string.value(),
            Literal::Bytes(bytes) => bytes_to_regex_string(bytes.value()),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Literal::Utf8(string) => string.span(),
            Literal::Bytes(bytes) => bytes.span(),
        }
    }
}

impl Parser {
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

    fn parse_attr(&mut self, attr: &mut Attribute) -> Option<AttributeParser> {
        let mut tokens = std::mem::replace(&mut attr.tokens, TokenStream::new()).into_iter();

        match tokens.next() {
            Some(TokenTree::Group(group)) => {
                Some(AttributeParser::new(group.stream()))
            },
            _ => None
        }
    }

    /// Try to parse the main `#[logos(...)]`, does nothing if
    /// the attribute's name isn't `logos`.
    pub fn try_parse_logos(&mut self, attr: &mut Attribute) {
        if !attr.path.is_ident("logos") {
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
                ("extras", NestedValue::Assign(value)) => {
                    let span = value.span();

                    if let MaybeVoid::Some(previous) = self.extras.replace(value) {
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
                },
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

        let mut def = Definition::new(literal);

        for (position, next) in nested.enumerate() {
            match next {
                Nested::Unexpected(tokens) => {
                    self.err("Unexpected token in attribute", tokens.span());
                },
                Nested::Unnamed(tokens) => match position {
                    0 => def.callback = self.parse_callback(tokens),
                    _ => {
                        self.err(
                            "\
                            Expected a named argument at this position\n\n\

                            Hint: If you are trying to define a callback here use: callback = ...\
                            ",
                            tokens.span()
                        );
                    }
                },
                Nested::Named(name, value) => {
                    def.named_attr(name, value, self);
                },
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
                self.err("Inline callbacks must use closure syntax with exactly one parameter", span);
                return None;
            }
        };

        let body = match tokens.next() {
            Some(TokenTree::Group(group)) => group.stream(),
            Some(first) => {
                let mut body = TokenStream::from(first);

                body.extend(tokens);
                body
            },
            None => {
                self.err("Callback missing a body", span);
                return None;
            }
        };

        let inline = InlineCallback {
            arg,
            body,
            span,
        };

        Some(inline.into())
    }

    /// Checks if `ty` is a declared generic param, if so replaces it
    /// with a concrete type defined using #[logos(type T = Type)]
    ///
    /// If no matching generic param is found, all lifetimes are fixed
    /// to the source lifetime
    pub fn get_type(&self, ty: &mut Type) -> TokenStream {
        if let Type::Path(tp) = ty {
            // Skip types that begin with `self::`
            if tp.qself.is_none() {
                // If `ty` is a generic type parameter, try to find
                // its concrete type defined with #[logos(type T = Type)]
                if let Some(substitue) = self.types.find(&tp.path) {
                    return substitue;
                }
            }
        }

        // If `ty` is a concrete type, fix its lifetimes to 'source
        replace_lifetimes(ty);
        quote!(#ty)
    }

    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Errors
    where
        M: Into<Cow<'static, str>>,
    {
        self.errors.err(message, span)
    }
}
