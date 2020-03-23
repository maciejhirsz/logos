use beef::lean::Cow;
use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens, TokenStreamExt};

#[derive(Debug)]
pub struct Error {
    message: Cow<'static, str>,
    span: Span,
}

impl Error {
    pub fn new<M>(message: M, span: Span) -> Self
    where
        M: Into<Cow<'static, str>>,
    {
        Error {
            message: message.into(),
            span,
        }
    }
}

impl ToTokens for Error {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let message = &*self.message;

        tokens.append_all(
            quote_spanned!(self.span => {
                compile_error!(#message)
            })
        )
    }
}
