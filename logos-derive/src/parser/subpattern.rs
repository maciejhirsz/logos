use syn::parse::{Parse, ParseStream};
use syn::punctuated::{Pair, Punctuated};
use syn::{MetaNameValue, Token};

pub struct SubpatternInput {
    kvs: Punctuated<MetaNameValue, Token![,]>,
}

impl Parse for SubpatternInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(SubpatternInput {
            kvs: Punctuated::parse_terminated(input)?,
        })
    }
}

impl SubpatternInput {
    pub fn into_iter(self) -> impl Iterator<Item = MetaNameValue> {
        self.kvs.into_pairs().map(Pair::into_value)
    }
}
