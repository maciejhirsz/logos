use proc_macro2::{TokenTree, TokenStream, Span, Spacing};
use quote::{quote, ToTokens};
use syn::{Expr, Ident, Lit};

/// Analog to Option<TokenStream>, except when put into the quote!
/// macro, `MaybeVoid::Void` will produce `()`
#[derive(Clone)]
pub enum MaybeVoid {
    Some(TokenStream),
    Void
}

impl Default for MaybeVoid {
    fn default() -> MaybeVoid {
        MaybeVoid::Void
    }
}

impl MaybeVoid {
    pub fn replace(&mut self, stream: TokenStream) -> MaybeVoid {
        std::mem::replace(self, MaybeVoid::Some(stream))
    }

    pub fn take(&mut self) -> MaybeVoid {
        std::mem::replace(self, MaybeVoid::Void)
    }
}

impl ToTokens for MaybeVoid {
    fn to_tokens(&self, out: &mut TokenStream) {
        match self {
            MaybeVoid::Some(stream) => out.extend(stream.clone()),
            MaybeVoid::Void => out.extend(quote!(())),
        }
    }

    fn to_token_stream(&self) -> TokenStream {
        match self {
            MaybeVoid::Some(stream) => stream.clone(),
            MaybeVoid::Void => quote!(()),
        }
    }

    fn into_token_stream(self) -> TokenStream {
        match self {
            MaybeVoid::Some(stream) => stream,
            MaybeVoid::Void => quote!(()),
        }
    }
}

pub fn is_punct(tt: &TokenTree, expect: char) -> bool {
    match tt {
        TokenTree::Punct(punct) if punct.as_char() == expect && punct.spacing() == Spacing::Alone => true,
        _ => false,
    }
}

/// If supplied `tt` is a punct matching a char, returns `None`, else returns `tt`
pub fn expect_punct(tt: Option<TokenTree>, expect: char) -> Option<TokenTree> {
    tt.filter(|tt| !is_punct(&tt, expect))
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

pub fn bytes_to_regex_string(bytes: Vec<u8>) -> String {
    if bytes.is_ascii() {
        unsafe {
            // Unicode values are prohibited, so we can't use
            // safe version of String::from_utf8
            //
            // We can, however, construct a safe ASCII string
            return String::from_utf8_unchecked(bytes);
        }
    }

    let mut string = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        if byte < 0x80 {
            string.push(byte as char);
        } else {
            static DIGITS: [u8; 16] = *b"0123456789abcdef";

            string.push_str(r"\x");
            string.push(DIGITS[(byte / 16) as usize] as char);
            string.push(DIGITS[(byte % 16) as usize] as char);
        }
    }

    string
}
