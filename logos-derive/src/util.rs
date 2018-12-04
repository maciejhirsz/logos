pub use syn::{Attribute, Lit, Ident, Meta, NestedMeta};
pub use proc_macro2::Span;
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

pub struct VariantDefinition {
    pub value: String,
    pub callback: Option<Ident>,
}

pub trait Value {
    fn value(value: String) -> Self;

    fn nested(&mut self, nested: &NestedMeta) {
        panic!("Unexpected nested attribute: {}", quote!(#nested));
    }
}

impl Value for String {
    fn value(value: String) -> Self {
        value
    }
}

impl Value for Ident {
    fn value(value: String) -> Self {
        ident(&value)
    }
}

impl Value for VariantDefinition {
    fn value(value: String) -> Self {
        VariantDefinition {
            value,
            callback: None,
        }
    }

    fn nested(&mut self, nested: &NestedMeta) {
        match nested {
            NestedMeta::Meta(Meta::NameValue(ref nval)) if nval.ident == "callback" => {
                let callback = match nval.lit {
                    Lit::Str(ref c) => ident(&c.value()),
                    ref lit => panic!("Invalid callback value: {}", quote!(#lit)),
                };

                self.callback.insert(callback, |_| panic!("Only one callback can be defined per variant definition!"));
            },
            _ => panic!("Unexpected nested attribute: {}", quote!(#nested)),
        }
    }
}

pub fn value_from_attr<V>(name: &str, attr: &Attribute) -> Option<V>
where
    V: Value,
{
    let meta = match attr.parse_meta() {
        Ok(meta) => meta,
        Err(_) => panic!("Couldn't parse attribute: {}", quote!(#attr)),
    };

    match meta {
        Meta::Word(ref ident) if ident == name => {
            panic!("Expected #[{} = ...], or #[{}(...)]", name, name);
        },
        Meta::NameValue(ref nval) if nval.ident == name => {
            let value = match nval.lit {
                Lit::Str(ref v) => v.value(),
                _ => panic!("#[{}] value must be a literal string", name),
            };

            Some(V::value(value))
        },
        Meta::List(ref list) if list.ident == name => {
            let mut iter = list.nested.iter();

            let value = match iter.next() {
                Some(NestedMeta::Literal(Lit::Str(ref v))) => v.value(),
                _ => panic!("#[{}] first argument must be a literal string, got: {}", name, quote!(#attr)),
            };

            let mut value = V::value(value);

            for nested in iter {
                value.nested(nested);
            }

            Some(value)
        },
        _ => None,
    }
}

pub fn ident(ident: &str) -> Ident {
    match syn::parse_str::<Ident>(ident) {
        Ok(ident) => ident,
        Err(_)    => panic!("Unable to parse {:?} into a Rust identifier.", ident),
    }
}
