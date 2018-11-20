//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

extern crate syn;
extern crate quote;
extern crate proc_macro;
extern crate proc_macro2;
extern crate regex_syntax;

mod util;
mod tree;
mod regex;
mod handlers;
mod generator;

use tree::{Node, Fork};
use util::OptionExt;
use handlers::Handlers;
use generator::Generator;

use quote::quote;
use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use syn::{ItemEnum, Fields, LitStr};

#[proc_macro_derive(Logos, attributes(error, end, token, regex))]
pub fn logos(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("#[token] can be only applied to enums");

    let size = item.variants.len();
    let name = &item.ident;

    let mut error = None;
    let mut end = None;

    let mut fork = Fork::default();

    for variant in &item.variants {
        if variant.discriminant.is_some() {
            panic!("`{}::{}` has a discriminant value set. This is not allowed for Tokens.", name, variant.ident);
        }

        match variant.fields {
            Fields::Unit => {},
            _ => panic!("`{}::{}` has fields. This is not allowed for Tokens.", name, variant.ident),
        }

        for attr in &variant.attrs {
            let ident = &attr.path.segments[0].ident;

            if ident == "error" {
                error.insert(&variant.ident, |_| panic!("Only one #[error] variant can be declared."));
            }

            if ident == "end" {
                end.insert(&variant.ident, |_| panic!("Only one #[end] variant can be declared."));
            }

            let token = ident == "token";
            let regex = ident == "regex";

            if token || regex {
                let mut tts = attr.tts.clone().into_iter();

                match tts.next() {
                    Some(TokenTree::Punct(ref punct)) if punct.as_char() == '=' => {},
                    Some(invalid) => panic!("#[token] Expected '=', got {}", invalid),
                    _ => panic!("Invalid token")
                }

                match tts.next() {
                    Some(TokenTree::Literal(literal)) => {
                        let path = syn::parse::<LitStr>(quote!{ #literal }.into())
                                        .expect("#[token] value must be a literal string")
                                        .value();

                        let node = if regex {
                            Node::from_regex(&path, &variant.ident)
                        } else {
                            Node::from_sequence(&path, &variant.ident)
                        };

                        fork.insert(node);
                    },
                    Some(invalid) => panic!("#[token] Invalid value: {}", invalid),
                    None => panic!("Invalid token")
                };

                assert!(tts.next().is_none(), "Unexpected token!");
            }
        }
    }

    // panic!("{:#?}", fork);

    let mut handlers = Handlers::new();

    for branch in fork.arms.drain(..) {
        handlers.insert(branch)
    }

    let error = match error {
        Some(error) => error,
        None => panic!("Missing #[error] token variant."),
    };

    let end = match end {
        Some(end) => end,
        None => panic!("Missing #[end] token variant.")
    };

    // panic!("{:#?}", handlers);

    let mut generator = Generator::new(name);

    let handlers = handlers.into_iter().map(|handler| {
        use handlers::Handler;

        match handler {
            Handler::Eof        => quote! { Some(eof) },
            Handler::Error      => quote! { Some(_error) },
            Handler::Whitespace => quote! { None },
            Handler::Tree(tree) => generator.print_tree(tree),
        }
    }).collect::<Vec<_>>();

    let fns = generator.fns();

    let tokens = quote! {
        impl ::logos::Logos for #name {
            type Extras = ();

            const SIZE: usize = #size;
            const ERROR: Self = #name::#error;

            fn lexicon<'a, S: ::logos::Source>() -> &'a ::logos::Lexicon<::logos::Lexer<Self, S>> {
                use ::logos::internal::LexerInternal;

                type Lexer<S> = ::logos::Lexer<#name, S>;

                fn eof<S: ::logos::Source>(lex: &mut Lexer<S>) {
                    lex.token = #name::#end;
                }

                fn _error<S: ::logos::Source>(lex: &mut Lexer<S>) {
                    lex.bump();

                    lex.token = #name::#error;
                }

                #fns

                &[#(#handlers),*]
            }
        }
    };

    // panic!("{}", tokens);

    TokenStream::from(tokens).into()
}
