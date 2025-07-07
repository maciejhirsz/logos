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
mod parser;
mod util;
mod pattern;

#[macro_use]
#[allow(missing_docs)]
mod macros;

use std::mem;

use error::{Error, Errors, SpannedError};
use generator::Generator;
use graph::Graph;
use leaf::{DisambiguationError, Leaf, Leaves};
use parser::{IgnoreFlags, Mode, Parser};
use pattern::Pattern;
use quote::ToTokens;
use regex_automata::dfa::dense::DFA;
use regex_syntax::escape;
use util::MaybeVoid;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::{Fields, ItemEnum};

const LOGOS_ATTR: &str = "logos";
const ERROR_ATTR: &str = "error";
const TOKEN_ATTR: &str = "token";
const REGEX_ATTR: &str = "regex";

/// Generate a `Logos` implementation for the given struct, provided as a stream of rust tokens.
pub fn generate(input: TokenStream) -> TokenStream {
    debug!("Reading input token streams");

    let mut item: ItemEnum = syn::parse2(input).expect("Logos can be only be derived for enums");
    let item_span = item.span();

    let name = &item.ident;

    let mut parser = Parser::default();

    for param in item.generics.params {
        parser.parse_generic(param);
    }

    for attr in &mut item.attrs {
        parser.try_parse_logos(attr);
    }

    let mut pats = Leaves::new();

    {
        let errors = &mut parser.errors;

        for skip in mem::take(&mut parser.skips) {

            // TODO Subpatterns
            // TODO custom priority

            let pattern = match Pattern::compile(&skip.literal) {
                Ok(pattern) => pattern,
                Err(err) => {
                    parser.err(err, skip.literal.span());
                    continue;
                }
            };

            let priority = pattern.priority();
            pats.push(
                Leaf::new_skip(skip.literal.span(), pattern)
                    .priority(priority)
                    .callback(Some(skip.into_callback())),
            );
        }
    }

    debug!("Iterating through enum variants");

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
        let leaf = move |span, pattern| Leaf::new(var_ident, span, pattern).field(field.clone());

        for attr in &mut variant.attrs {
            let attr_name = match attr.path().get_ident() {
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
                TOKEN_ATTR => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[token(...)]", attr.span());
                            continue;
                        }
                    };

                    if !definition.ignore_flags.is_empty() {
                        // TODO
                    }
                    // TODO subpatterns

                    let pattern = match Pattern::compile_lit(&definition.literal) {
                        Ok(pattern) => pattern,
                        Err(err) => {
                            parser.err(err, definition.literal.span());
                            continue;
                        }
                    };

                    let literal_len = match &definition.literal {
                        parser::Literal::Utf8(lit_str) => lit_str.value().len(),
                        parser::Literal::Bytes(lit_byte_str) => lit_byte_str.value().len(),
                    };

                    pats.push(
                    leaf(definition.literal.span(), pattern)
                        .priority(definition.priority.unwrap_or(literal_len * 2))
                        .callback(definition.callback),
                    );
                }
                REGEX_ATTR => {
                    let definition = match parser.parse_definition(attr) {
                        Some(definition) => definition,
                        None => {
                            parser.err("Expected #[regex(...)]", attr.span());
                            continue;
                        }
                    };

                    if !definition.ignore_flags.is_empty() {
                        // TODO
                    }
                    // TODO subpatterns
                    // TODO custom priority

                    let pattern = match Pattern::compile(&definition.literal) {
                        Ok(pattern) => pattern,
                        Err(err) => {
                            parser.err(err, definition.literal.span());
                            continue;
                        }
                    };

                    let pat_priority = pattern.priority();
                    pats.push(
                    leaf(definition.literal.span(), pattern)
                        .priority(definition.priority.unwrap_or(pat_priority))
                        .callback(definition.callback),
                    );
                }
                _ => (),
            }
        }
    }

    debug!("Parsing additional options (extras, source, ...)");

    let error_type = parser.error_type.take();
    let extras = parser.extras.take();
    let source = parser
        .source
        .take()
        .map(strip_wrapping_parens)
        .unwrap_or(match parser.mode {
            Mode::Utf8 => quote!(str),
            Mode::Binary => quote!([u8]),
        });
    let logos_path = parser
        .logos_path
        .take()
        .unwrap_or_else(|| parse_quote!(::logos));

    let generics = parser.generics();
    let this = quote!(#name #generics);

    let impl_logos = |body| {
        quote! {
            impl<'s> #logos_path::Logos<'s> for #this {
                type Error = #error_type;

                type Extras = #extras;

                type Source = #source;

                fn lex(lex: &mut #logos_path::Lexer<'s, Self>) -> Option<Result<Self, Self::Error>> {
                    #body
                }
            }
        }
    };

    debug!("Checking if any two tokens have the same priority");

    for DisambiguationError(a, b) in pats.errors() {
        let a = &pats[a];
        let b = &pats[b];
        let disambiguate = a.priority + 1;

        let mut err = |a: &Leaf, b: &Leaf| {
            parser.err(
                format!(
                    "\
                    A definition of variant `{a}` can match the same input as another definition of variant `{b}`.\n\
                    \n\
                    hint: Consider giving one definition a higher priority: \
                    #[{attr}(..., priority = {disambiguate})]\
                    ",
                    attr = match a.callback {
                        Some(_) => "regex",
                        None => "skip"
                    }
                ),
                a.span
            );
        };

        err(a, b);
        err(b, a);
    }

    if let Some(errors) = parser.errors.render() {
        return impl_logos(errors);
    }

    #[cfg(feature = "debug")]
    {
        // TODO
        // debug!("Generating graphs");
        //
        // if let Some(path) = parser.export_dir {
        //     let path = std::path::Path::new(&path);
        //     let dir = if path.extension().is_none() {
        //         path
        //     } else {
        //         path.parent().unwrap_or(std::path::Path::new(""))
        //     };
        //     match std::fs::create_dir_all(dir) {
        //         Ok(()) => {
        //             if path.extension() == Some(std::ffi::OsStr::new("dot"))
        //                 || path.extension().is_none()
        //             {
        //                 match graph.get_dot() {
        //                     Ok(s) => {
        //                         let dot_path = if path.extension().is_none() {
        //                             path.join(format!("{}.dot", name.to_string().to_lowercase()))
        //                         } else {
        //                             path.to_path_buf()
        //                         };
        //                         if let Err(e) = std::fs::write(dot_path, s) {
        //                             debug!("Error writing dot graph: {}", e);
        //                         }
        //                     }
        //                     Err(e) => {
        //                         debug!("Error generating dot graph: {}", e);
        //                     }
        //                 }
        //             }
        //
        //             if path.extension() == Some(std::ffi::OsStr::new("mmd"))
        //                 || path.extension().is_none()
        //             {
        //                 match graph.get_mermaid() {
        //                     Ok(s) => {
        //                         let mermaid_path = if path.extension().is_none() {
        //                             path.join(format!("{}.mmd", name.to_string().to_lowercase()))
        //                         } else {
        //                             path.to_path_buf()
        //                         };
        //                         if let Err(e) = std::fs::write(mermaid_path, s) {
        //                             debug!("Error writing mermaid graph: {}", e);
        //                         }
        //                     }
        //                     Err(e) => {
        //                         debug!("Error generating mermaid graph: {}", e);
        //                     }
        //                 }
        //             }
        //         }
        //         Err(e) => {
        //             debug!("Error creating graph export dir: {}", e);
        //         }
        //     }
        // }
    }

    debug!("Generating graph from pats:\n{pats:#?}");

    let graph = match Graph::try_new(pats) {
        Ok(nfa) => nfa,
        Err(msg) => {
            let mut errors = Errors::default();
            errors.err(msg, item_span);
            return impl_logos(errors.render().unwrap())
        }
    };

    debug!("Generating code from graph:\n{:#?}", graph.dfa());

    let generator = Generator::new(name, &this, &graph);

    let body = generator.generate();
    impl_logos(quote! {
        use #logos_path::internal::LexerInternal;

        type Lexer<'s> = #logos_path::Lexer<'s, #this>;

        #body
    })
}

/// Strip all logos attributes from the given struct, allowing it to be used in code without `logos-derive` present.
pub fn strip_attributes(input: TokenStream) -> TokenStream {
    let mut item: ItemEnum = syn::parse2(input).expect("Logos can be only be derived for enums");

    strip_attrs_from_vec(&mut item.attrs);

    for attr in &mut item.attrs {
        if let syn::Meta::List(meta) = &mut attr.meta {
            if meta.path.is_ident("derive") {
                let mut tokens =
                    std::mem::replace(&mut meta.tokens, TokenStream::new()).into_iter();

                while let Some(TokenTree::Ident(ident)) = tokens.next() {
                    let punct = tokens.next();

                    if ident == "Logos" {
                        continue;
                    }

                    meta.tokens.extend([TokenTree::Ident(ident)]);
                    meta.tokens.extend(punct);
                }
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
    attr.path().is_ident(LOGOS_ATTR)
        || attr.path().is_ident(TOKEN_ATTR)
        || attr.path().is_ident(REGEX_ATTR)
}

fn strip_wrapping_parens(t: TokenStream) -> TokenStream {
    let tts: Vec<TokenTree> = t.into_iter().collect();

    if tts.len() != 1 {
        tts.into_iter().collect()
    } else {
        match tts.into_iter().next().unwrap() {
            TokenTree::Group(g) => {
                if g.delimiter() == Delimiter::Parenthesis {
                    g.stream()
                } else {
                    core::iter::once(TokenTree::Group(g)).collect()
                }
            }
            tt => core::iter::once(tt).collect(),
        }
    }
}
