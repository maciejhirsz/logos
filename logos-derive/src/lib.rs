//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

mod generator;
mod error;
mod graph;
mod util;
mod leaf;

use error::Error;
use generator::Generator;
use graph::{Graph, Fork, Rope};
use leaf::Leaf;
use util::{Literal, Definition};

use beef::lean::Cow;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, Fields, ItemEnum};
use syn::spanned::Spanned;

enum Mode {
    Utf8,
    Binary,
}

#[proc_macro_derive(
    Logos,
    attributes(logos, extras, error, end, token, regex, extras, callback)
)]
pub fn logos(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("#[token] can be only applied to enums");
    let super_span = item.span();

    let size = item.variants.len();
    let name = &item.ident;

    let mut extras: Option<Ident> = None;
    let mut error = None;
    let mut end = None;
    let mut mode = Mode::Utf8;
    let mut errors = Vec::new();
    let mut trivia = Some((true, Cow::borrowed(r"[ \t\f]"), Span::call_site()));

    for attr in &item.attrs {
        if let Some(ext) = util::value_from_attr("extras", attr) {
            if let Some(_) = extras.replace(ext) {
                errors.push(Error::new("Only one #[extras] attribute can be declared.").span(super_span));
            }
        }

        if let Some(nested) = util::read_attr("logos", attr) {
            for item in nested {
                if let Some(t) = util::value_from_nested::<Option<Literal>>("trivia", item) {
                    trivia = match t {
                        Some(Literal::Utf8(string, span)) => {
                            Some((true, string.into(), span))
                        },
                        Some(Literal::Bytes(bytes, span)) => {
                            mode = Mode::Binary;

                            Some((false, util::bytes_to_regex_string(&bytes).into(), span))
                        },
                        None => None,
                    };
                }
            }
        }
    }

    let mut variants = Vec::new();
    let mut ropes = Vec::new();
    let mut regex_ids = Vec::new();
    let mut graph = Graph::new();

    for variant in &item.variants {
        variants.push(&variant.ident);

        let span = variant.span();

        if variant.discriminant.is_some() {
            errors.push(Error::new(
                format!(
                    "`{}::{}` has a discriminant value set. This is not allowed for Tokens.",
                    name,
                    variant.ident,
                ),
            ).span(span));
        }

        match variant.fields {
            Fields::Unit => {}
            _ => {
                errors.push(Error::new(
                    format!(
                        "`{}::{}` has fields. This is not allowed for Tokens.",
                        name, variant.ident
                    ),
                ).span(span));
            }
        }

        for attr in &variant.attrs {
            let ident = &attr.path.segments[0].ident;
            let variant = &variant.ident;

            if ident == "error" {
                if let Some(previous) = error.replace(variant) {
                    errors.extend(vec![
                        Error::new("Only one #[error] variant can be declared.").span(span),
                        Error::new("Previously declared #[error]:").span(previous.span()),
                    ]);
                }
            }

            if ident == "end" {
                if let Some(previous) = end.replace(variant) {
                    errors.extend(vec![
                        Error::new("Only one #[end] variant can be declared.").span(span),
                        Error::new("Previously declared #[end]:").span(previous.span()),
                    ]);
                }
            }

            let mut with_definition = |definition: Definition<Literal>| {
                let token = Leaf::token(variant).callback(definition.callback);

                if let Literal::Bytes(..) = definition.value {
                    mode = Mode::Binary;
                }

                (token, definition.value)
            };

            if let Some(definition) = util::value_from_attr("token", attr) {
                let (token, value) = with_definition(definition);

                let value = value.into_bytes();
                let then = graph.push(token.priority(value.len()));

                ropes.push(Rope::new(value, then));
            } else if let Some(definition) = util::value_from_attr("regex", attr) {
                let (token, value) = with_definition(definition);

                let then = graph.reserve();

                let (utf8, regex, span) = match value {
                    Literal::Utf8(string, span) => (true, string, span),
                    Literal::Bytes(bytes, span) => {
                        mode = Mode::Binary;

                        (false, util::bytes_to_regex_string(&bytes), span)
                    }
                };

                match graph.regex(utf8, &regex, then.get()) {
                    Ok((len, mut id)) => {
                        let then = graph.insert(then, token.priority(len));
                        regex_ids.push(id);

                        // Drain recursive miss values.
                        // We need the root node to have straight branches.
                        while let Some(miss) = graph[id].miss() {
                            if miss == then {
                                errors.push(
                                    Error::new("#[regex]: expression can match empty string.\n\n\
                                                hint: consider changing * to +").span(span)
                                );
                                break;
                            } else {
                                regex_ids.push(miss);
                                id = miss;
                            }
                        }
                    },
                    Err(err) => errors.push(err.span(span)),
                }
            }

        //     if let Some(callback) = value_from_attr("callback", attr) {
        //         generator.set_callback(variant, callback);
        //     }
        }
    }

    let mut root = Fork::new();

    if let Some((utf8, regex, span)) = trivia {
        let then = graph.push(Leaf::Trivia);

        match graph.regex(utf8, &*regex, then) {
            Ok((_, id)) => {
                let trivia = graph.fork_off(id);

                root.merge(trivia, &mut graph);
            },
            Err(err) => errors.push(err.span(span)),
        }
    }

    if error.is_none() {
        errors.push(Error::new("missing #[error] token variant.").span(super_span));
    }

    if end.is_none() {
        errors.push(Error::new("missing #[end] token variant.").span(super_span));
    }

    if errors.len() > 0 {
        return quote! {
            fn _logos_derive_compile_errors() {
                #(#errors)*
            }
        }.into();
    }

    let error = error.expect("Already checked for none above; qed");
    let end = end.expect("Already checked for none above; qed");
    let extras = match extras {
        Some(ext) => quote!(#ext),
        None => quote!(()),
    };
    let source = match mode {
        Mode::Utf8 => quote!(Source),
        Mode::Binary => quote!(BinarySource),
    };

    for id in regex_ids {
        let fork = graph.fork_off(id);
        root.merge(fork, &mut graph);
    }
    for rope in ropes {
        root.merge(rope.into_fork(&mut graph), &mut graph)
    }
    let root = graph.push(root);

    graph.shake(root);

    // panic!("{:#?}\n\n{} nodes", graph, graph.nodes().iter().filter_map(|n| n.as_ref()).count());

    let mut generator = Generator::new(name, root, &graph);

    let body = generator.generate();

    let tokens = quote! {
        impl ::logos::Logos for #name {
            type Extras = #extras;

            const SIZE: usize = #size;
            const ERROR: Self = #name::#error;
            const END: Self = #name::#end;

            fn lex<'source, Source>(lex: &mut ::logos::Lexer<#name, Source>)
            where
                Source: ::logos::Source<'source>,
                Self: ::logos::source::WithSource<Source>,
            {
                use ::logos::internal::LexerInternal;
                use ::logos::source::{Source as Src, Split};

                type Lexer<S> = ::logos::Lexer<#name, S>;

                fn _end<'s, S: Src<'s>>(lex: &mut Lexer<S>) {
                    lex.token = #name::#end;
                }

                fn _error<'s, S: Src<'s>>(lex: &mut Lexer<S>) {
                    lex.bump(1);

                    lex.token = #name::#error;
                }

                #body
            }
        }

        impl<'source, Source: ::logos::source::#source<'source>> ::logos::source::WithSource<Source> for #name {}
    };

    // panic!("{}", tokens);

    TokenStream::from(tokens)
}
