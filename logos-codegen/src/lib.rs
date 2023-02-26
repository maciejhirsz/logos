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
use quote::ToTokens;
use util::MaybeVoid;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::{Fields, ItemEnum};

const LOGOS_ATTR: &str = "logos";
const EXTRAS_ATTR: &str = "extras";
const ERROR_ATTR: &str = "error";
const END_ATTR: &str = "end";
const TOKEN_ATTR: &str = "token";
const REGEX_ATTR: &str = "regex";

/// Generate a `Logos` implementation for the given struct, provided as a stream of rust tokens.
pub fn generate(input: TokenStream) -> TokenStream {
    let mut item: ItemEnum = syn::parse2(input).expect("Logos can be only be derived for enums");

    let name = &item.ident;

    let mut parser = Parser::default();

    for param in item.generics.params {
        parser.parse_generic(param);
    }

    for attr in &mut item.attrs {
        parser.try_parse_logos(attr);

        // TODO: Remove in future versions
        if attr.path.is_ident(EXTRAS_ATTR) {
            parser.err(
                "\
                #[extras] attribute is deprecated. Use #[logos(extras = Type)] instead.\n\
                \n\
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
                ERROR_ATTR => {
                    // TODO: Remove in future versions
                    parser.err(
                        "\
                        Since 0.13 Logos no longer requires the #[error] variant.\n\
                        \n\
                        For help with migration see release notes: \
                        https://github.com/maciejhirsz/logos/releases\
                        ",
                        attr.span(),
                    );
                }
                END_ATTR => {
                    // TODO: Remove in future versions
                    parser.err(
                        "\
                        Since 0.11 Logos no longer requires the #[end] variant.\n\
                        \n\
                        For help with migration see release notes: \
                        https://github.com/maciejhirsz/logos/releases\
                        ",
                        attr.span(),
                    );
                }
                TOKEN_ATTR => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[token(...)]", attr.span());
                            continue;
                        }
                    };

                    if definition.ignore_flags.is_empty() {
                        let bytes = definition.literal.to_bytes();
                        let then = graph.push(
                            leaf(definition.literal.span())
                                .priority(definition.priority.unwrap_or(bytes.len() * 2))
                                .callback(definition.callback),
                        );

                        ropes.push(Rope::new(bytes, then));
                    } else {
                        let mir = definition
                            .literal
                            .escape_regex()
                            .to_mir(
                                &Default::default(),
                                definition.ignore_flags,
                                &mut parser.errors,
                            )
                            .expect("The literal should be perfectly valid regex");

                        let then = graph.push(
                            leaf(definition.literal.span())
                                .priority(definition.priority.unwrap_or_else(|| mir.priority()))
                                .callback(definition.callback),
                        );
                        let id = graph.regex(mir, then);

                        regex_ids.push(id);
                    }
                }
                REGEX_ATTR => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[regex(...)]", attr.span());
                            continue;
                        }
                    };
                    let mir = match definition.literal.to_mir(
                        &parser.subpatterns,
                        definition.ignore_flags,
                        &mut parser.errors,
                    ) {
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

    let error_type = parser.error_type.take();
    let extras = parser.extras.take();
    let source = match parser.mode {
        Mode::Utf8 => quote!(str),
        Mode::Binary => quote!([u8]),
    };
    let logos_path = parser
        .logos_path
        .take()
        .unwrap_or_else(|| parse_quote!(::logos));

    let generics = parser.generics();
    let this = quote!(#name #generics);

    let impl_logos = |body| {
        quote! {
            impl<'s> ::logos::Logos<'s> for #this {
                type Error = #error_type;

                type Extras = #extras;

                type Source = #source;

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
                    A definition of variant `{0}` can match the same input as another definition of variant `{1}`.\n\
                    \n\
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
        use #logos_path::internal::{LexerInternal, CallbackResult};

        type Lexer<'s> = #logos_path::Lexer<'s, #this>;

        fn _end<'s>(lex: &mut Lexer<'s>) {
            lex.end()
        }

        fn _error<'s>(lex: &mut Lexer<'s>) {
            lex.bump_unchecked(1);

            lex.error();
        }

        #body
    });

    // panic!("{}", tokens);

    tokens
}

/// Strip all logos attributes from the given struct, allowing it to be used in code without `logos-derive` present.
pub fn strip_attributes(input: TokenStream) -> TokenStream {
    let mut item: ItemEnum = syn::parse2(input).expect("Logos can be only be derived for enums");

    strip_attrs_from_vec(&mut item.attrs);

    for attr in &mut item.attrs {
        if attr.path.is_ident("derive") {
            if let Ok(syn::Meta::List(mut meta)) = attr.parse_meta() {
                meta.nested = meta.nested.into_iter().filter(|nested| !matches!(nested, syn::NestedMeta::Meta(nested) if nested.path().is_ident("Logos"))).collect();

                attr.tokens = TokenStream::new();
                meta.paren_token.surround(&mut attr.tokens, |tokens| {
                    meta.nested.to_tokens(tokens);
                });
            }
        }
    }

    for variant in &mut item.variants {
        strip_attrs_from_vec(&mut variant.attrs);
        for field in &mut variant.fields {
            strip_attrs_from_vec(&mut field.attrs);
        }
    }

    item.to_token_stream()
}

fn strip_attrs_from_vec(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| !is_logos_attr(attr))
}

fn is_logos_attr(attr: &syn::Attribute) -> bool {
    attr.path.is_ident(LOGOS_ATTR)
        || attr.path.is_ident(EXTRAS_ATTR)
        || attr.path.is_ident(END_ATTR)
        || attr.path.is_ident(TOKEN_ATTR)
        || attr.path.is_ident(REGEX_ATTR)
}
