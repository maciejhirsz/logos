pub use syn::{Attribute, LitStr, Ident, PathArguments, Lifetime};
pub use proc_macro2::{Span, TokenTree};
use quote::quote;

pub trait OptionExt<T> {
    fn insert(&mut self, val: T, f: impl FnOnce(&T));
}

impl<T> OptionExt<T> for Option<T> {
    fn insert(&mut self, val: T, f: impl FnOnce(&T)) {
        match self {
            Some(t) => f(t),
            slot    => *slot = Some(val),
        }
    }
}

pub fn value_from_attr(name: &str, attr: &Attribute) -> Option<String> {
    if &attr.path.segments[0].ident == name {
        let mut tts = attr.tts.clone().into_iter();

        match tts.next() {
            Some(TokenTree::Punct(ref punct)) if punct.as_char() == '=' => {},
            Some(invalid) => panic!("#[{}] Expected '=', got {}", name, invalid),
            _ => panic!("Invalid token")
        }

        let value = match tts.next() {
            Some(TokenTree::Literal(literal)) => {
                match syn::parse::<LitStr>(quote!{ #literal }.into()) {
                    Ok(v)  => v.value(),
                    Err(_) => panic!("#[{}] value must be a literal string", name),
                }
            },
            Some(invalid) => panic!("#[extras] Invalid value: {}", invalid),
            None => panic!("Invalid token")
        };

        assert!(tts.next().is_none(), "Unexpected token!");

        Some(value)
    } else {
        None
    }
}

pub fn ident(ident: &str) -> Ident {
    match syn::parse_str::<Ident>(ident) {
        Ok(ident) => ident,
        Err(_)    => panic!("Unable to parse {:?} into a Rust identifier.", ident),
    }
}
