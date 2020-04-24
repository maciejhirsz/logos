use syn::parse::{Parse, ParseStream};
use syn::punctuated::{Pair, Punctuated};
use syn::{Ident, LitStr, MetaNameValue, Token};

use crate::error::{Error, Result};
use crate::mir::Mir;

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

pub struct Subpattern {
    invoke: String,
    produce: String,
}

impl Subpattern {
    fn new(name: &Ident, lit: &str) -> Self {
        Subpattern {
            invoke: format!("(?&{})", name),
            produce: format!("(?:{})", lit),
        }
    }

    pub fn utf8(name: &Ident, lit: &str) -> Result<Self> {
        let _ = Mir::utf8(lit)?;
        Ok(Self::new(name, lit))
    }

    pub fn binary(name: &Ident, lit: &str) -> Result<Self> {
        let _ = Mir::binary(lit)?;
        Ok(Self::new(name, lit))
    }

    pub fn fix<'a>(&self, pattern: String) -> String {
        pattern.replace(&self.invoke, &self.produce)
    }
}
