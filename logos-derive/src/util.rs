pub use proc_macro2::{TokenStream, Span};
use quote::quote;
pub use syn::{Attribute, Ident, Lit, Meta, NestedMeta};

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
    pub callback: Option<Ident>,
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

    fn nested(&mut self, nested: &NestedMeta) {
        panic!("Unexpected nested attribute: {}", quote!(#nested));
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

    fn nested(&mut self, nested: &NestedMeta) {
        match nested {
            NestedMeta::Meta(Meta::NameValue(ref nval)) if nval.path.is_ident("callback") => {
                let callback = match nval.lit {
                    Lit::Str(ref c) => ident(&c.value()),
                    ref lit => panic!("Invalid callback value: {}", quote!(#lit)),
                };

                self.callback.insert(callback, |_| {
                    panic!("Only one callback can be defined per variant definition!")
                });
            }
            _ => panic!("Unexpected nested attribute: {}", quote!(#nested)),
        }
    }
}

pub fn read_attr(name: &str, attr: &Attribute) -> Option<Vec<NestedMeta>> {
    let meta = match attr.parse_meta() {
        Ok(meta) => meta,
        Err(_) => panic!("Couldn't parse attribute: {}", quote!(#attr)),
    };

    read_meta(name, meta)
}

// pub fn read_nested(name: &str, nested: NestedMeta) -> Option<Vec<NestedMeta>> {
//     if let NestedMeta::Meta(meta) = nested {
//         read_meta(name, meta)
//     } else {
//         None
//     }
// }

pub fn read_meta(name: &str, meta: Meta) -> Option<Vec<NestedMeta>> {
    match meta {
        Meta::Path(ref path) if path.is_ident(name) => {
            panic!("Expected #[{} = ...], or #[{}(...)]", name, name);
        }
        Meta::NameValue(nval) => {
            if nval.path.is_ident(name) {
                Some(vec![NestedMeta::Lit(nval.lit)])
            } else {
                None
            }
        }
        Meta::List(list) => {
            if list.path.is_ident(name) {
                Some(list.nested.into_iter().collect())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn value_from_attr<V>(name: &str, attr: &Attribute) -> Option<V>
where
    V: Value,
{
    read_attr(name, attr).map(parse_value)
}

// pub fn value_from_nested<V>(name: &str, nested: NestedMeta) -> Option<V>
// where
//     V: Value,
// {
//     read_nested(name, nested).map(parse_value)
// }

fn parse_value<V>(items: Vec<NestedMeta>) -> V
where
    V: Value,
{
    let mut iter = items.iter();

    let value = match iter.next() {
        Some(NestedMeta::Lit(Lit::Str(ref v))) => Some(Literal::Utf8(v.value(), v.span())),
        Some(NestedMeta::Lit(Lit::ByteStr(ref v))) => Some(Literal::Bytes(v.value(), v.span())),
        _ => None,
    };

    let mut value = V::value(value);

    for nested in iter {
        value.nested(nested);
    }

    value
}

pub fn ident(ident: &str) -> Ident {
    match syn::parse_str::<Ident>(ident) {
        Ok(ident) => ident,
        Err(_) => panic!("Unable to parse {:?} into a Rust identifier.", ident),
    }
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

// pub struct MergeAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
// {
//     left: Peekable<L>,
//     right: Peekable<R>,
// }

// impl<L, R> MergeAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
// {
//     pub fn new<LI, RI>(left: LI, right: RI) -> Self
//     where
//         LI: IntoIterator<IntoIter = L, Item = L::Item>,
//         RI: IntoIterator<IntoIter = R, Item = R::Item>,
//     {
//         MergeAscending {
//             left: left.into_iter().peekable(),
//             right: right.into_iter().peekable(),
//         }
//     }
// }

// impl<L, R> Iterator for MergeAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
//     L::Item: Ord,
// {
//     type Item = L::Item;

//     fn next(&mut self) -> Option<L::Item> {
//         let which = match (self.left.peek(), self.right.peek()) {
//             (Some(l), Some(r)) => Some(l.cmp(r)),
//             (Some(_), None) => Some(Ordering::Less),
//             (None, Some(_)) => Some(Ordering::Greater),
//             (None, None) => None,
//         };

//         match which {
//             Some(Ordering::Less) => self.left.next(),
//             Some(Ordering::Equal) => {
//                 // Advance both
//                 self.left.next();
//                 self.right.next()
//             }
//             Some(Ordering::Greater) => self.right.next(),
//             None => None,
//         }
//     }
// }

// pub struct DiffAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
// {
//     left: Peekable<L>,
//     right: Peekable<R>,
// }

// impl<L, R> DiffAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
// {
//     pub fn new<LI, RI>(left: LI, right: RI) -> Self
//     where
//         LI: IntoIterator<IntoIter = L, Item = L::Item>,
//         RI: IntoIterator<IntoIter = R, Item = R::Item>,
//     {
//         DiffAscending {
//             left: left.into_iter().peekable(),
//             right: right.into_iter().peekable(),
//         }
//     }
// }

// impl<L, R> Iterator for DiffAscending<L, R>
// where
//     L: Iterator<Item = R::Item>,
//     R: Iterator,
//     L::Item: Ord,
// {
//     type Item = L::Item;

//     fn next(&mut self) -> Option<L::Item> {
//         let which = match (self.left.peek(), self.right.peek()) {
//             (Some(l), Some(r)) => Some(l.cmp(r)),
//             (Some(_), None) => Some(Ordering::Less),
//             (None, Some(_)) => Some(Ordering::Greater),
//             (None, None) => None,
//         };

//         match which {
//             Some(Ordering::Less) => self.left.next(),
//             Some(Ordering::Equal) => {
//                 // Advance both to skip matches
//                 self.left.next();
//                 self.right.next();

//                 self.next()
//             }
//             Some(Ordering::Greater) => {
//                 // Skip right side
//                 self.right.next();

//                 self.next()
//             }
//             None => None,
//         }
//     }
// }
