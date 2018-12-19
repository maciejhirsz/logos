use std::cmp::Ordering;
use std::iter::Peekable;

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

pub struct MergeAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
{
    left: Peekable<L>,
    right: Peekable<R>,
}

impl<L, R> MergeAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
{
    pub fn new<LI, RI>(left: LI, right: RI) -> Self
    where
        LI: IntoIterator<IntoIter = L, Item = L::Item>,
        RI: IntoIterator<IntoIter = R, Item = R::Item>,
    {
        MergeAscending {
            left: left.into_iter().peekable(),
            right: right.into_iter().peekable(),
        }
    }
}

impl<L, R> Iterator for MergeAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
    L::Item: Ord,
{
    type Item = L::Item;

    fn next(&mut self) -> Option<L::Item> {
        let which = match (self.left.peek(), self.right.peek()) {
            (Some(l), Some(r)) => Some(l.cmp(r)),
            (Some(_), None) => Some(Ordering::Less),
            (None, Some(_)) => Some(Ordering::Greater),
            (None, None) => None,
        };

        match which {
            Some(Ordering::Less) => self.left.next(),
            Some(Ordering::Equal) => {
                // Advance both
                self.left.next();
                self.right.next()
            },
            Some(Ordering::Greater) => self.right.next(),
            None => None,
        }
    }
}

pub struct DiffAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
{
    left: Peekable<L>,
    right: Peekable<R>,
}

impl<L, R> DiffAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
{
    pub fn new<LI, RI>(left: LI, right: RI) -> Self
    where
        LI: IntoIterator<IntoIter = L, Item = L::Item>,
        RI: IntoIterator<IntoIter = R, Item = R::Item>,
    {
        DiffAscending {
            left: left.into_iter().peekable(),
            right: right.into_iter().peekable(),
        }
    }
}

impl<L, R> Iterator for DiffAscending<L, R>
where
    L: Iterator<Item = R::Item>,
    R: Iterator,
    L::Item: Ord,
{
    type Item = L::Item;

    fn next(&mut self) -> Option<L::Item> {
        let which = match (self.left.peek(), self.right.peek()) {
            (Some(l), Some(r)) => Some(l.cmp(r)),
            (Some(_), None) => Some(Ordering::Less),
            (None, Some(_)) => Some(Ordering::Greater),
            (None, None) => None,
        };

        match which {
            Some(Ordering::Less) => self.left.next(),
            Some(Ordering::Equal) => {
                // Advance both to skip matches
                self.left.next();
                self.right.next();

                self.next()
            },
            Some(Ordering::Greater) => self.right.next(),
            None => None,
        }
    }
}
