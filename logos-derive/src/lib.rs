//! <img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.svg?sanitize=true" alt="Logos logo" width="250" align="right">
//!
//! # Logos
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]
#![doc(html_logo_url = "https://maciej.codes/kosz/logos.png")]

mod error;
mod generator;
mod graph;
mod leaf;
mod mir;
mod parser;
mod util;

use generator::Generator;
use graph::{DisambiguationError, Fork, Graph, Rope};
use leaf::Leaf;
use parser::{Mode, Parser};
use util::MaybeVoid;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Fields, ItemEnum};

#[proc_macro_derive(Logos, attributes(logos, extras, error, end, token, regex))]
pub fn logos(input: TokenStream) -> TokenStream {
    let mut item: ItemEnum = syn::parse(input).expect("Logos can be only be derived for enums");

    let mut size = item.variants.len();
    let name = &item.ident;

    let mut error = None;
    let mut parser = Parser::default();

    for param in item.generics.params {
        parser.parse_generic(param);
    }

    for attr in &mut item.attrs {
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

    let mut ropes = Vec::new();
    let mut regex_ids = Vec::new();
    let mut graph = Graph::new();

    for variant in &mut item.variants {
        if let Some((_, expr)) = variant.discriminant.take() {
            let expr = expr.into_token_stream();
            let value = expr.to_string().parse().unwrap_or(usize::max_value());

            if value >= size {
                // Size must always be 1 greater than highest discriminant value
                size = value + 1;
            }
        }

        let field = match &mut variant.fields {
            Fields::Unit => MaybeVoid::Void,
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    parser.err(
                        format!(
                            "Logos currently only supports variants with one field, found {}",
                            fields.unnamed.len(),
                        ),
                        fields.span(),
                    );
                }

                let ty = &mut fields
                    .unnamed
                    .first_mut()
                    .expect("Already checked len; qed")
                    .ty;
                let ty = parser.get_type(ty);

                MaybeVoid::Some(ty)
            }
            Fields::Named(fields) => {
                parser.err("Logos doesn't support named fields yet.", fields.span());

                MaybeVoid::Void
            }
        };

        // Lazy leaf constructor to avoid cloning
        let var_ident = &variant.ident;
        let leaf = move |span| Leaf::new(var_ident, span).field(field.clone());

        for attr in &mut variant.attrs {
            let attr_name = match attr.path.get_ident() {
                Some(ident) => ident.to_string(),
                None => continue,
            };

            match attr_name.as_str() {
                "error" => {
                    let span = variant.ident.span();
                    if let Some(previous) = error.replace(&variant.ident) {
                        parser
                            .err("Only one #[error] variant can be declared.", span)
                            .err("Previously declared #[error]:", previous.span());
                    }
                }
                "end" => {
                    // TODO: Remove in future versions
                    parser.err(
                        "\
                        Since 0.11 Logos no longer requires the #[end] variant.\n\n\

                        For help with migration see release notes: \
                        https://github.com/maciejhirsz/logos/releases\
                        ",
                        attr.span(),
                    );
                }
                "token" => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[token(...)]", attr.span());
                            continue;
                        }
                    };

                    let bytes = definition.literal.to_bytes();
                    let then = graph.push(
                        leaf(definition.literal.span())
                            .priority(definition.priority.unwrap_or(bytes.len() * 2))
                            .callback(definition.callback),
                    );

                    ropes.push(Rope::new(bytes, then));
                }
                "regex" => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[regex(...)]", attr.span());
                            continue;
                        }
                    };
                    let mir = match definition
                        .literal
                        .to_mir(&parser.subpatterns, &mut parser.errors)
                    {
                        Ok(mir) => mir,
                        Err(err) => {
                            parser.err(err, definition.literal.span());
                            continue;
                        }
                    };
                    let then = graph.push(
                        leaf(definition.literal.span())
                            .priority(definition.priority.unwrap_or_else(|| mir.priority()))
                            .callback(definition.callback),
                    );
                    let id = graph.regex(mir, then);

                    regex_ids.push(id);
                }
                _ => (),
            }
        }
    }

    let mut root = Fork::new();

    let extras = parser.extras.take();
    let source = match parser.mode {
        Mode::Utf8 => quote!(str),
        Mode::Binary => quote!([u8]),
    };

    let error_def = match error {
        Some(error) => Some(quote!(const ERROR: Self = #name::#error;)),
        None => {
            parser.err("missing #[error] token variant.", Span::call_site());
            None
        }
    };

    let generics = parser.generics();
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

    for id in regex_ids {
        let fork = graph.fork_off(id);

        root.merge(fork, &mut graph);
    }
    for rope in ropes {
        root.merge(rope.into_fork(&mut graph), &mut graph);
    }
    while let Some(id) = root.miss.take() {
        let fork = graph.fork_off(id);

        if fork.branches().next().is_some() {
            root.merge(fork, &mut graph);
        } else {
            break;
        }
    }

    for &DisambiguationError(a, b) in graph.errors() {
        let a = graph[a].unwrap_leaf();
        let b = graph[b].unwrap_leaf();
        let disambiguate = a.priority + 1;

        let mut err = |a: &Leaf, b: &Leaf| {
            parser.err(
                format!(
                    "\
                    A definition of variant `{0}` can match the same input as another definition of variant `{1}`.\n\n\

                    hint: Consider giving one definition a higher priority: \
                    #[regex(..., priority = {2})]\
                    ",
                    a.ident,
                    b.ident,
                    disambiguate,
                ),
                a.span
            );
        };

        err(a, b);
        err(b, a);
    }

    if let Some(errors) = parser.errors.render() {
        return impl_logos(errors).into();
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
