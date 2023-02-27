use std::fmt;

use beef::lean::Cow;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::{quote_spanned, ToTokens, TokenStreamExt};

use crate::parse::ParseError;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Errors {
    collected: Vec<ParseError>,
}

impl Default for Errors {
    fn default() -> Self {
        Errors {
            collected: Vec::new(),
        }
    }
}

impl Errors {
    pub fn err<M>(&mut self, message: M, span: Span) -> &mut Self
    where
        M: Into<Cow<'static, str>>,
    {
        self.collected.push(ParseError::new(message, span));

        self
    }

    pub fn render(self) -> Option<TokenStream> {
        let errors = self.collected;

        match errors.len() {
            0 => None,
            _ => Some(quote! {
                fn _logos_derive_compile_errors() {
                    #(#errors)*
                }
            }),
        }
    }
}

pub struct Error(Cow<'static, str>);

impl Error {
    pub fn new<M>(message: M) -> Self
    where
        M: Into<Cow<'static, str>>,
    {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<regex_syntax::Error> for Error {
    fn from(err: regex_syntax::Error) -> Error {
        Error(err.to_string().into())
    }
}

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Error {
        Error(err.into())
    }
}

impl From<Error> for Cow<'static, str> {
    fn from(err: Error) -> Self {
        err.0
    }
}

impl ToTokens for ParseError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let message = &*self.msg;

        tokens.append_all(quote_spanned!(self.span.into() => {
            compile_error!(#message)
        }))
    }
}
