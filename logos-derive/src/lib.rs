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
extern crate utf8_ranges;

mod util;
mod tree;
mod regex;
mod handlers;
mod generator;

use tree::{Node, Fork};
use util::{OptionExt, value_from_attr};
use handlers::Handlers;
use generator::Generator;

use quote::quote;
use proc_macro::TokenStream;
use syn::{ItemEnum, Fields};

#[proc_macro_derive(Logos, attributes(
    extras,
    error,
    end,
    token,
    regex,
    extras,
    callback,
))]
pub fn logos(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("#[token] can be only applied to enums");

    let size = item.variants.len();
    let name = &item.ident;

    let mut extras = None;
    let mut error = None;
    let mut end = None;

    for attr in &item.attrs {
        if let Some(ext) = value_from_attr("extras", attr) {
            extras.insert(util::ident(&ext), |_| panic!("Only one #[extras] attribute can be declared."));
        }
    }

    // Initially we pack all variants into a single fork, this is where all the logic branching
    // magic happens.
    let mut fork = Fork::default();

    // Then the fork is split into handlers using all possible permutations of the first byte of
    // any branch as the index of a 256-entries-long table.
    let mut handlers = Handlers::new();

    // Finally the `Generator` will spit out Rust code for all the handlers.
    let mut generator = Generator::new(name);

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
            let variant = &variant.ident;

            if ident == "error" {
                error.insert(variant, |_| panic!("Only one #[error] variant can be declared."));
            }

            if ident == "end" {
                end.insert(variant, |_| panic!("Only one #[end] variant can be declared."));
            }

            if let Some(path) = value_from_attr("token", attr) {
                fork.insert(Node::from_sequence(&path, variant));
            }

            if let Some(path) = value_from_attr("regex", attr) {
                fork.insert(Node::from_regex(&path, variant));
            }

            if let Some(callback) = value_from_attr("callback", attr) {
                generator.set_callback(variant, util::ident(&callback));
            }
        }
    }

    // panic!("{:#?}", fork);

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

    let extras = match extras {
        Some(ext) => quote!(#ext),
        None      => quote!(()),
    };

    // panic!("{:#?}", handlers);

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
            type Extras = #extras;

            const SIZE: usize = #size;
            const ERROR: Self = #name::#error;

            fn lexicon<'lexicon, 'source, S: ::logos::Source<'source>>() -> &'lexicon ::logos::Lexicon<::logos::Lexer<Self, S>> {
                use ::logos::internal::LexerInternal;

                type Lexer<S> = ::logos::Lexer<#name, S>;

                fn eof<'source, S: ::logos::Source<'source>>(lex: &mut Lexer<S>) {
                    lex.token = #name::#end;
                }

                fn _error<'source, S: ::logos::Source<'source>>(lex: &mut Lexer<S>) {
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
