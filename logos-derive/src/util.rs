use proc_macro2::{TokenTree, Span, Spacing};
use syn::{Expr, Ident, Lit};

use crate::leaf::Callback;

pub struct Definition {
    pub literal: Literal,
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

pub fn is_punct(tt: &TokenTree, expect: char) -> bool {
    match tt {
        TokenTree::Punct(punct) if punct.as_char() == expect && punct.spacing() == Spacing::Alone => true,
        _ => false,
    }
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
