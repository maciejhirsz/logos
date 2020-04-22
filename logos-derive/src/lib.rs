//! <img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.svg?sanitize=true" alt="Logos logo" width="250" align="right">
//!
//! # Logos
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

mod generator;
mod error;
mod graph;
mod util;
mod leaf;
mod parser;
mod parsers;
mod type_params;

use error::Error;
use generator::Generator;
use graph::{Graph, Fork, Rope};
use leaf::Leaf;
use util::{Literal, Definition};
use parser::Parser;
use type_params::TypeParams;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Fields, ItemEnum, GenericParam};
use syn::spanned::Spanned;

enum Mode {
    Utf8,
    Binary,
}

#[proc_macro_derive(
    Logos,
    attributes(logos, extras, error, end, token, regex, extras)
)]
pub fn logos(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("#[token] can be only applied to enums");
    let super_span = item.span();

    let size = item.variants.len();
    let name = &item.ident;

    let mut error = None;
    let mut mode = Mode::Utf8;
    let mut parser = Parser::new();
    let mut type_params = TypeParams::default();

    for param in item.generics.params {
        match param {
            GenericParam::Lifetime(lt) => {
                type_params.explicit_lifetime(lt, &mut parser.errors);
            },
            GenericParam::Type(ty) => {
                type_params.add_param(ty.ident);
            },
            GenericParam::Const(_) => {
                let span = param.span();

                parser.err("Logos doesn't support const generics.", span);
            }
        }
    }

    let generics = type_params.generics(&mut parser.errors);

    for attr in &item.attrs {
        parser.try_parse_logos(attr);

        // TODO: Remove in future versions
        if attr.path.is_ident("extras") {
            parser.err(
                "\
                #[extras] attribute is deprecated. Use #[logos(extras = Type)] instead.\n\n\

                For help with migration see release notes: \
                https://github.com/maciejhirsz/logos/releases\
                ",
                attr.span(),
            );
        }
    }

    let mut variants = Vec::new();
    let mut ropes = Vec::new();
    let mut regex_ids = Vec::new();
    let mut graph = Graph::new();

    for variant in &item.variants {
        variants.push(&variant.ident);

        let span = variant.span();

        if let Some((_, value)) = &variant.discriminant {
            let span = value.span();
            let value = util::unpack_int(value).unwrap_or(usize::max_value());

            if value >= size {
                parser.errors.push(Error::new(
                    format!(
                        "Discriminant value for `{}` is invalid. Expected integer in range 0..={}.",
                        variant.ident,
                        size,
                    ),
                ).span(span));
            }
        }

        let field = match &variant.fields {
            Fields::Unit => None,
            Fields::Unnamed(ref fields) => {
                if fields.unnamed.len() != 1 {
                    parser.errors.push(Error::new(
                        format!(
                            "Logos currently only supports variants with one field, found {}",
                            fields.unnamed.len(),
                        )
                    ).span(fields.span()))
                }

                let field = fields.unnamed.first().expect("Already checked len; qed").ty.clone();

                Some(field)
            }
            Fields::Named(_) => {
                parser.errors.push(Error::new("Logos doesn't support named fields yet.").span(span));

                None
            }
        };

        for attr in &variant.attrs {
            let variant = &variant.ident;

            let mut with_definition = |definition: Definition<Literal>| {
                if let Literal::Bytes(..) = definition.value {
                    mode = Mode::Binary;
                }

                (
                    Leaf::token(variant).field(field.clone()).callback(definition.callback),
                    definition.value,
                )
            };

            if attr.path.is_ident("error") {
                if let Some(previous) = error.replace(variant) {
                    parser
                        .err("Only one #[error] variant can be declared.", span)
                        .err("Previously declared #[error]:", previous.span());
                }
            } else if attr.path.is_ident("end") {
                parser.err(
                    "Since 0.11 Logos no longer requires the #[end] variant.\n\n\

                    For help with migration see release notes: https://github.com/maciejhirsz/logos/releases",
                    attr.span(),
                );
            } else if attr.path.is_ident("token") {
                match util::value_from_attr("token", attr) {
                    Ok(definition) => {
                        let (token, value) = with_definition(definition);

                        let value = value.into_bytes();
                        let then = graph.push(token.priority(value.len()));

                        ropes.push(Rope::new(value, then));
                    },
                    Err(err) => parser.errors.push(err),
                }
            } else if attr.path.is_ident("regex") {
                match util::value_from_attr("regex", attr) {
                    Ok(definition) => {
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
                                        parser.err(
                                            "#[regex]: expression can match empty string.\n\n\
                                             hint: consider changing * to +",
                                            span,
                                        );
                                        break;
                                    } else {
                                        regex_ids.push(miss);
                                        id = miss;
                                    }
                                }
                            },
                            Err(err) => parser.errors.push(err.span(span)),
                        }
                    },
                    Err(err) => parser.errors.push(err),
                }
            }
        }
    }

    let mut root = Fork::new();

    let extras = parser.extras();
    let source = match mode {
        Mode::Utf8 => quote!(str),
        Mode::Binary => quote!([u8]),
    };

    let error_def = match error {
        Some(error) => Some(quote!(const ERROR: Self = #name::#error;)),
        None => {
            parser.err("missing #[error] token variant.", super_span);
            None
        },
    };

    let this = quote!(#name #generics);

    let impl_logos = |body| {
        quote! {
            impl<'s> ::logos::Logos<'s> for #this {
                type Extras = #extras;

                type Source = #source;

                const SIZE: usize = #size;

                #error_def

                fn lex(lex: &mut ::logos::Lexer<'s, Self>) {
                    #body
                }
            }
        }
    };

    if let Some(errors) = parser.errors.render() {
        return impl_logos(errors).into();
    }

    for id in regex_ids {
        let fork = graph.fork_off(id);
        root.merge(fork, &mut graph);
    }
    for rope in ropes {
        root.merge(rope.into_fork(&mut graph), &mut graph)
    }
    while let Some(id) = root.miss.take() {
        let fork = graph.fork_off(id);

        if fork.branches().next().is_some() {
            root.merge(fork, &mut graph);
        } else {
            break;
        }
    }
    let root = graph.push(root);

    graph.shake(root);

    // panic!("{:#?}\n\n{} nodes", graph, graph.nodes().iter().filter_map(|n| n.as_ref()).count());

    let generator = Generator::new(name, &this, root, &graph);

    let body = generator.generate();
    let tokens = impl_logos(quote! {
        use ::logos::internal::{LexerInternal, CallbackResult};

        type Lexer<'s> = ::logos::Lexer<'s, #this>;

        fn _end<'s>(lex: &mut Lexer<'s>) {
            lex.end()
        }

        fn _error<'s>(lex: &mut Lexer<'s>) {
            lex.bump_unchecked(1);

            lex.set(#name::#error);
        }

        #body
    });

    // panic!("{}", tokens);

    TokenStream::from(tokens)
}
