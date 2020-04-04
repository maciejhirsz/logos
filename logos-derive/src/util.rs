use proc_macro2::{TokenStream, TokenTree, Span, Spacing};
use quote::{quote, TokenStreamExt};
use syn::{Attribute, Expr, Ident, Lit, Meta, NestedMeta};
use syn::spanned::Spanned;

use crate::error::{Error, SpannedError};

type Result<T> = std::result::Result<T, SpannedError>;

pub trait OptionExt<T> {
    fn insert(&mut self, val: T, f: impl FnOnce(&T));
}

impl<T> OptionExt<T> for Option<T> {
    fn insert(&mut self, val: T, f: impl FnOnce(&T)) {
        match self {
            Some(t) => f(t),
            slot => *slot = Some(val),
        }
    }
}

pub struct Definition<V: Value> {
    pub value: V,
    pub callback: Option<TokenStream>,
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

    fn nested(&mut self, nested: TokenStream) -> Result<()> {
        Err(Error::new("Unexpected nested attribute").span(nested.span()))
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
                panic!("Expected a string, got a bytes instead: {:02X?}", bytes)
            }
            None => panic!("Expected a string"),
        }
    }
}

impl Value for Ident {
    fn value(value: Option<Literal>) -> Self {
        ident(&String::value(value))
    }
}

impl<V: Value> Value for Definition<V> {
    fn value(value: Option<Literal>) -> Self {
        Definition {
            value: V::value(value),
            callback: None,
        }
    }

    fn nested(&mut self, nested: TokenStream) -> Result<()> {
        if let Some(prev) = &self.callback {
            return Err(Error::new("Only one callback can be defined").span(prev.span()));
        }

        self.callback = Some(nested);

        Ok(())
    }
}

pub fn read_attr(name: &str, attr: &Attribute) -> Result<Option<TokenStream>> {
    if !attr.path.is_ident(name) {
        return Ok(None);
    }

    let stream = attr_fields(name, attr.tokens.clone(), attr.span())?;

    Ok(Some(stream))
}

fn attr_fields<Tokens>(name: &str, stream: Tokens, span: Span) -> Result<TokenStream>
where
    Tokens: IntoIterator<Item = TokenTree>,
{
    let mut tokens = stream.into_iter();

    match tokens.next() {
        Some(tt) if is_punct(&tt, '=') => {
            match tokens.next() {
                None => return Err(Error::new("Expected value after =").span(tt.span())),
                Some(next) => Ok(next.into()),
            }
        },
        Some(TokenTree::Group(group)) => {
            Ok(group.stream())
        }
        _ => {
            let err = format!("Expected #[{} = ...] or #[{}(...)]", name, name);

            Err(Error::new(err).span(span))
        }
    }
}

pub fn value_from_attr<V>(name: &str, attr: &Attribute) -> Result<Option<V>>
where
    V: Value,
{
    read_attr(name, attr)?.map(parse_value).transpose()
}

pub fn value_from_nested<V>(name: &str, nested: TokenStream) -> Result<Option<V>>
where
    V: Value,
{
    let span = nested.span();
    let mut iter = nested.into_iter();

    match iter.next() {
        Some(TokenTree::Ident(ident)) if ident == name => (),
        _ => return Ok(None),
    };


    let stream = attr_fields(name, iter, span)?;

    parse_value(stream).map(Some)
}

fn is_punct(tt: &TokenTree, expect: char) -> bool {
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
                TokenTree::Ident(_) => tt.into(),
                tt => {
                    return Err(Error::new("Expected an function label or an inline callback").span(tt.span()));
                }
            };

            value.nested(nested)?;
        }
    }

    Ok(value)
}

fn parse_inline_callback(tokens: &mut impl Iterator<Item = TokenTree>, span: Span) -> Result<TokenStream> {
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

    Ok(quote!({
        #[inline]
        fn callback<'s>(#ident: &mut Lexer<'s>) -> impl CallbackResult<Product = ()> {
            #body
        }

        callback
    }))
}

pub fn ident(ident: &str) -> Ident {
    match syn::parse_str::<Ident>(ident) {
        Ok(ident) => ident,
        Err(_) => panic!("Unable to parse {:?} into a Rust identifier.", ident),
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
