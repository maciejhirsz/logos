//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

// mod generator;
// mod handlers;
mod error;
mod graph;
mod util;
mod token;

use error::Error;

// use self::generator::Generator;
// use self::handlers::{Handler, Handlers, Trivia};
// use self::tree::{Fork, Leaf, Node};
// use self::util::{value_from_attr, Definition, Literal, OptionExt};
// use regex::Regex;
use graph::{Graph, Fork, Rope};
use token::Token;
use util::{Literal, Definition};

use proc_macro::TokenStream;
use quote::quote;
use syn::{Fields, ItemEnum};
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

    // let size = item.variants.len();
    let name = &item.ident;

    // let mut extras: Option<Ident> = None;
    let mut error = None;
    let mut end = None;
    let mut mode = Mode::Utf8;
    // let mut trivia = Trivia::Default;

    // Initially we pack all variants into a single fork, this is where all the logic branching
    // magic happens.
    // let mut fork = Fork::default();

    // for attr in &item.attrs {
    //     if let Some(ext) = value_from_attr("extras", attr) {
    //         extras.insert(ext, |_| {
    //             panic!("Only one #[extras] attribute can be declared.")
    //         });
    //     }

    //     if let Some(nested) = util::read_attr("logos", attr) {
    //         for item in nested {
    //             if let Some(t) = util::value_from_nested::<Option<Literal>>("trivia", item) {
    //                 panic!("{:?}", t);
    //                 let (utf8, regex) = match t {
    //                     Some(Literal::Utf8(string)) => (true, string),
    //                     Some(Literal::Bytes(bytes)) => {
    //                         mode = Mode::Binary;

    //                         (false, util::bytes_to_regex_string(&bytes))
    //                     }
    //                     None => {
    //                         match trivia {
    //                             Trivia::Patterns(_) => {}
    //                             Trivia::Default => trivia = Trivia::Patterns(vec![]),
    //                         }

    //                         continue;
    //                     }
    //                 };

    //                 let node = Node::from_regex(&regex, utf8);

    //                 match node {
    //                     Node::Branch(ref branch)
    //                         if branch.then.is_none() && branch.regex.len() == 1 =>
    //                     {
    //                         let pattern = branch.regex.first().clone();

    //                         match trivia {
    //                             Trivia::Patterns(ref mut patterns) => patterns.push(pattern),
    //                             Trivia::Default => trivia = Trivia::Patterns(vec![pattern]),
    //                         }

    //                         continue;
    //                     }
    //                     _ => {}
    //                 }

    //                 fork.insert(node.leaf(Leaf::Trivia));
    //             }
    //         }
    //     }
    // }

    let mut variants = Vec::new();
    let mut ropes = Vec::new();
    let mut regex_ids = Vec::new();
    let mut errors = Vec::new();
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
                let id = graph.push(Token::new(variant));

                if let Some((_, previous)) = error.replace((id, span)) {
                    errors.extend(vec![
                        Error::new("Only one #[error] variant can be declared.").span(span),
                        Error::new("Previously declared #[error]:").span(previous),
                    ]);
                }
            }

            if ident == "end" {
                let id = graph.push(Token::new(variant));

                if let Some((_, previous)) = end.replace((id, span)) {
                    errors.extend(vec![
                        Error::new("Only one #[end] variant can be declared.").span(span),
                        Error::new("Previously declared #[end]:").span(previous),
                    ]);
                }
            }

            let mut with_definition = |definition: Definition<Literal>| {
                let token = Token::new(variant).callback(definition.callback);

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

                let then = graph.push(token);

                let (utf8, regex, span) = match value {
                    Literal::Utf8(string, span) => (true, string, span),
                    Literal::Bytes(bytes, span) => {
                        mode = Mode::Binary;

                        (false, util::bytes_to_regex_string(&bytes), span)
                    }
                };

                match graph.regex(utf8, &regex, then) {
                    Ok(id) => {
                        // Check if the regex can go to the leaf on a miss
                        if graph[id].miss() == Some(then) {
                            errors.push(
                                Error::new("#[regex]: expression can match empty string.\n\n\
                                            hint: consider changing * to +").span(span)
                            );
                        }
                        regex_ids.push(id);
                    },
                    Err(err) => errors.push(err.span(span)),
                }
            }

        //     if let Some(callback) = value_from_attr("callback", attr) {
        //         generator.set_callback(variant, callback);
        //     }
        }
    }

    if errors.len() > 0 {
        return quote! {
            fn _logos_derive_compile_errors() {
                #(#errors)*
            }
        }.into();
    }

    // graph.shake();
    // let count = graph.nodes().iter().filter_map(|n| n.as_ref()).count();
    // panic!("{:#?}\n\n{} nodes", graph, count);

    let mut root = Fork::new();

    for id in regex_ids {
        let fork = graph.fork_off(id);
        root.merge(fork, &mut graph);
    }
    for rope in ropes {
        root.merge(rope.into_fork(&mut graph), &mut graph)
    }

    graph.push(root);
    graph.shake();

    let count = graph.nodes().iter().filter_map(|n| n.as_ref()).count();

    panic!("{:#?}\n\n{} nodes", graph, count);

    // panic!("END");
}


// //// OLD CODE //// //

//     let error = match error {
//         Some(error) => error,
//         None => panic!("Missing #[error] token variant."),
//     };

//     let end = match end {
//         Some(end) => end,
//         None => panic!("Missing #[end] token variant."),
//     };

//     let extras = match extras {
//         Some(ext) => quote!(#ext),
//         None => quote!(()),
//     };

//     // panic!("{:#?}", handlers);

//     let handlers = handlers
//         .into_iter()
//         .map(|handler| match handler {
//             Handler::Error => quote!(Some(_error)),
//             Handler::Whitespace => quote!(None),
//             Handler::Tree(tree) => generator.print_tree(tree),
//         })
//         .collect::<Vec<_>>();

//     let fns = generator.fns();

//     let source = match mode {
//         Mode::Utf8 => quote!(Source),
//         Mode::Binary => quote!(BinarySource),
//     };

//     let tokens = quote! {
//         impl ::logos::Logos for #name {
//             type Extras = #extras;

//             const SIZE: usize = #size;
//             const ERROR: Self = #name::#error;
//             const END: Self = #name::#end;

//             fn lexicon<'lexicon, 'source, Source>() -> &'lexicon ::logos::Lexicon<::logos::Lexer<Self, Source>>
//             where
//                 Source: ::logos::Source<'source>,
//                 Self: ::logos::source::WithSource<Source>,
//             {
//                 use ::logos::internal::LexerInternal;
//                 use ::logos::source::Split;

//                 type Lexer<S> = ::logos::Lexer<#name, S>;

//                 fn _error<'source, S: ::logos::Source<'source>>(lex: &mut Lexer<S>) {
//                     lex.bump(1);

//                     lex.token = #name::#error;
//                 }

//                 #fns

//                 &[#(#handlers),*]
//             }
//         }

//         impl<'source, Source: ::logos::source::#source<'source>> ::logos::source::WithSource<Source> for #name {}
//     };

//     // panic!("{}", tokens);

//     TokenStream::from(tokens)
// }
