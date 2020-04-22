use proc_macro2::{TokenStream, TokenTree, Span, Spacing};
use quote::{quote, TokenStreamExt};
use syn::{Attribute, Expr, Ident, Lit};
use syn::spanned::Spanned;

use crate::leaf::Callback;
use crate::error::{Error, SpannedError, Errors};

type Result<T> = std::result::Result<T, SpannedError>;

pub struct Definition<V: Value> {
    pub value: V,
    pub callback: Callback,
}

#[derive(Debug)]
pub enum Literal {
    Utf8(String, Span),
    Bytes(Vec<u8>, Span),
}

impl Literal {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Literal::Utf8(string, _) => string.into_bytes(),
            Literal::Bytes(bytes, _) => bytes,
        }
    }
}

pub trait Value {
    fn value(value: Option<Literal>) -> Self;

    fn nested(&mut self, nested: Callback) -> Result<()> {
        let span = match nested {
            Callback::Label(label) => label.span(),
            Callback::Inline(arg, ..) => arg.span(),
            _ => return Ok(()),
        };

        Err(Error::new("Unexpected nested attribute").span(span))
    }
}

impl Value for Literal {
    fn value(value: Option<Literal>) -> Self {
        match value {
            Some(value) => value,
            None => panic!("Expected a string or bytes to be the first field to attribute."),
        }
    }
}

impl Value for Option<Literal> {
    fn value(value: Option<Literal>) -> Self {
        value
    }
}

impl Value for String {
    fn value(value: Option<Literal>) -> Self {
        match value {
            Some(Literal::Utf8(value, _)) => value,

            // TODO: Better errors
            Some(Literal::Bytes(bytes, _)) => {
                panic!("Expected a string, got bytes instead: {:02X?}", bytes)
            }
            None => panic!("Expected a string"),
        }
    }
}

impl Value for Ident {
    fn value(value: Option<Literal>) -> Self {
        String::value(value).to_ident()
    }
}

impl<V: Value> Value for Definition<V> {
    fn value(value: Option<Literal>) -> Self {
        Definition {
            value: V::value(value),
            callback: Callback::None,
        }
    }

    fn nested(&mut self, nested: Callback) -> Result<()> {
        match self.callback.span() {
            Some(span) => {
                return Err(Error::new("Only one callback can be defined").span(span));
            },
            _ => (),
        }

        self.callback = nested;

        Ok(())
    }
}

pub fn read_attr(name: &str, attr: &Attribute, errors: &mut Errors) -> TokenStream {
    let mut tokens = attr.tokens.clone().into_iter();

    match tokens.next() {
        Some(TokenTree::Group(group)) => group.stream(),
        _ => {
            errors.err(format!("Expected #[{}(...)]", name), attr.span());

            TokenStream::new()
        }
    }
}

fn attr_fields<Tokens>(name: &str, stream: Tokens, span: Span) -> Result<TokenStream>
where
    Tokens: IntoIterator<Item = TokenTree>,
{
    let mut tokens = stream.into_iter();

    match tokens.next() {
        Some(tt) if is_punct(&tt, '=') => {
            match tokens.next() {
                None => Err(Error::new("Expected value after =").span(tt.span())),
                Some(next) => {
                    let err = format!(
                        "#[{} = ...] definitions are not supported since v0.11.\n\n\

                        Use instead: #[{}({})]\n",
                        name,
                        name,
                        next,
                    );

                    Err(Error::new(err).span(span))
                }
            }
        },
        Some(TokenTree::Group(group)) => {
            Ok(group.stream())
        }
        _ => {
            let err = format!("Expected #[{}(...)]", name);

            Err(Error::new(err).span(span))
        }
    }
}

pub fn value_from_attr<V>(name: &str, attr: &Attribute) -> Result<V>
where
    V: Value,
{
    let stream = attr_fields(name, attr.tokens.clone(), attr.span())?;

    parse_value(stream)
}

pub fn is_punct(tt: &TokenTree, expect: char) -> bool {
    match tt {
        TokenTree::Punct(punct) if punct.as_char() == expect && punct.spacing() == Spacing::Alone => true,
        _ => false,
    }
}

fn parse_value<V>(items: TokenStream) -> Result<V>
where
    V: Value,
{
    let mut iter = items.into_iter();

    let value = match iter.next() {
        Some(TokenTree::Literal(lit)) => {
            match Lit::new(lit) {
                Lit::Str(ref v) => Some(Literal::Utf8(v.value(), v.span())),
                Lit::ByteStr(ref v) => Some(Literal::Bytes(v.value(), v.span())),
                _ => None,
            }
        },
        _ => None,
    };

    let mut value = V::value(value);

    while let Some(tt) = iter.next() {
        if !is_punct(&tt, ',') {
            return Err(Error::new("Expected ,").span(tt.span()));
        }

        while let Some(tt) = iter.next() {
            if is_punct(&tt, ',') {
                break;
            }

            let nested = match tt {
                tt if is_punct(&tt, '|') => parse_inline_callback(&mut iter, tt.span())?,
                TokenTree::Ident(label) => {
                    let mut out = quote!(#label);

                    while let Some(tt) = iter.next() {
                        if is_punct(&tt, ',') {
                            break;
                        }

                        out.append(tt);
                    }

                    Callback::Label(out)
                },
                tt => {
                    return Err(Error::new("Expected an function label or an inline callback").span(tt.span()));
                }
            };

            value.nested(nested)?;
        }
    }

    Ok(value)
}

fn parse_inline_callback(tokens: &mut impl Iterator<Item = TokenTree>, span: Span) -> Result<Callback> {
    let ident = match tokens.next() {
        Some(TokenTree::Ident(ident)) => ident,
        _ => return Err(Error::new("Expected identifier following this token").span(span)),
    };

    match tokens.next() {
        Some(tt) if is_punct(&tt, '|') => (),
        _ => return Err(Error::new("Expected | following this token").span(ident.span())),
    }

    let body = match tokens.next() {
        Some(TokenTree::Group(group)) => group.stream(),
        first => {
            let mut body = quote!(#first);

            body.extend(tokens.take_while(|tt| !is_punct(tt, ',')));
            body
        }
    };

    Ok(Callback::Inline(ident, body))
}

pub trait ToIdent {
    fn to_ident(&self) -> Ident;
}

impl ToIdent for str {
    fn to_ident(&self) -> Ident {
        Ident::new(self, Span::call_site())
    }
}

pub fn unpack_int(expr: &Expr) -> Option<usize> {
    if let Expr::Lit(expr_lit) = expr {
        if let Lit::Int(int) = &expr_lit.lit {
            return int.base10_parse().ok();
        }
    }
    None
}

pub fn bytes_to_regex_string(bytes: &[u8]) -> String {
    let mut string = String::with_capacity(bytes.len());

    for &byte in bytes {
        if byte < 0x7F {
            string.push(byte as char);
        } else {
            string.push_str(&format!("\\x{:02x}", byte));
        }
    }

    string
}
