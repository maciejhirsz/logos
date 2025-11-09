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
mod pattern;
mod util;

#[macro_use]
#[allow(missing_docs)]
mod macros;

use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;

use error::Errors;
use generator::Generator;
use graph::{DisambiguationError, Graph};
use leaf::Leaf;
use parser::Parser;
use pattern::Pattern;
use quote::ToTokens;

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_quote, LitBool};
use syn::{Fields, ItemEnum};

use crate::graph::Config;
use crate::leaf::VariantKind;
use crate::parser::{ErrorType, Subpatterns};

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

    debug!("Iterating through subpatterns and skips");

    let utf8_mode = parser
        .utf8_mode
        .as_ref()
        .map(LitBool::value)
        .unwrap_or(true);
    let config = Config { utf8_mode };
    let subpatterns = Subpatterns::new(&parser.subpatterns, utf8_mode, &mut parser.errors);

    let mut pats = Vec::new();

    for skip in parser.skips.drain(..) {
        let Some(pattern_source) = subpatterns.subst_subpatterns(
            &skip.literal.escape(false),
            skip.literal.span(),
            &mut parser.errors,
        ) else {
            continue;
        };

        let pattern = match Pattern::compile(
            false,
            &pattern_source,
            skip.literal.token().to_string(),
            skip.literal.unicode(),
            false,
        ) {
            Ok(pattern) => pattern,
            Err(err) => {
                parser.errors.err(err, skip.literal.span());
                continue;
            }
        };

        let default_priority = pattern.priority();
        pats.push(
            Leaf::new(skip.literal.span(), pattern)
                .priority(skip.priority.unwrap_or(default_priority))
                .callback(skip.into_callback()),
        );
    }

    debug!("Iterating through enum variants");

    for variant in &mut item.variants {
        let var_ident = variant.ident.clone();

        let var_kind = match &mut variant.fields {
            Fields::Unit => VariantKind::Unit(var_ident),
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

                VariantKind::Value(var_ident, ty)
            }
            Fields::Named(fields) => {
                parser.err("Logos doesn't support named fields yet.", fields.span());

                VariantKind::Skip
            }
        };

        for attr in &mut variant.attrs {
            let attr_name = match attr.path().get_ident() {
                Some(ident) => ident.to_string(),
                None => continue,
            };

            match attr_name.as_str() {
                ERROR_ATTR => {
                    // TODO: Remove in future versions
                    parser.err(
                        concat!(
                            "Since 0.13 Logos no longer requires the #[error] variant.",
                            "\n\n",
                            "For help with migration see release notes: ",
                            "https://github.com/maciejhirsz/logos/releases"
                        ),
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

                    let pattern_res = if definition.ignore_flags.ignore_case {
                        let pattern_src = definition.literal.escape(true);
                        Pattern::compile(
                            true,
                            &pattern_src,
                            definition.literal.token().to_string(),
                            definition.literal.unicode(),
                            true,
                        )
                    } else {
                        Pattern::compile_lit(&definition.literal)
                    };

                    let pattern = match pattern_res {
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
                        Leaf::new(definition.literal.span(), pattern)
                            .variant_kind(var_kind.clone())
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

                    let Some(pattern_source) = subpatterns.subst_subpatterns(
                        &definition.literal.escape(false),
                        definition.literal.span(),
                        &mut parser.errors,
                    ) else {
                        continue;
                    };

                    let unicode = definition.literal.unicode();
                    let ignore_case = definition.ignore_flags.ignore_case;
                    let pattern = match Pattern::compile(
                        false,
                        &pattern_source,
                        definition.literal.token().to_string(),
                        unicode,
                        ignore_case,
                    ) {
                        Ok(pattern) => pattern,
                        Err(err) => {
                            parser.err(err, definition.literal.span());
                            continue;
                        }
                    };

                    let allow_greedy = definition.allow_greedy.unwrap_or(false);
                    if !allow_greedy && pattern.check_for_greedy_all() {
                        parser.err(concat!(
                            "This pattern contains an unbounded greedy dot repetition (.* or .+). ",
                            "This will cause the entirety of the input to be read for every token. ",
                            "Consider making your repetition non-greedy or changing it to a more ",
                            "specific character class. If this is the intended behavior, add ",
                            "#[regex(..., allow_greedy = true)]"
                        ), definition.literal.span());
                    }

                    let default_priority = pattern.priority();
                    pats.push(
                        Leaf::new(definition.literal.span(), pattern)
                            .variant_kind(var_kind.clone())
                            .priority(definition.priority.unwrap_or(default_priority))
                            .callback(definition.callback),
                    );
                }
                _ => (),
            }
        }
    }

    debug!("Parsing additional options (extras, utf8, ...)");

    let ErrorType {
        ty: error_type,
        callback: error_callback,
    } = parser.error_type.take().unwrap_or_default();
    let extras = parser.extras.take();
    let non_utf8_pats = pats
        .iter()
        .filter(|leaf| !leaf.pattern.hir().properties().is_utf8())
        .collect::<Vec<_>>();
    if utf8_mode && !non_utf8_pats.is_empty() {
        // If utf8 mode is specified, make sure no patterns match illegal utf8
        for leaf in non_utf8_pats {
            parser.err(format!(concat!(
                "UTF-8 mode is requested, but the pattern {} of variant `{}` can match invalid utf8.\n",
                "You can disable UTF-8 mode with #[logos(utf8 = false)]"
            ), leaf.pattern.source(), leaf.kind), leaf.span);
        }
    };

    let source = match utf8_mode {
        true => quote!(str),
        false => quote!([u8]),
    };
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

                fn lex(lex: &mut #logos_path::Lexer<'s, Self>)
                    -> std::option::Option<std::result::Result<Self, <Self as #logos_path::Logos<'s>>::Error>> {
                    #body
                }
            }
        }
    };

    if cfg!(feature = "debug") {
        let leaves_rendered = pats
            .iter()
            .enumerate()
            .map(|(leaf_id, leaf)| format!("  {}: {} (priority: {})", leaf_id, leaf, leaf.priority))
            .collect::<Vec<_>>()
            .join("\n");
        debug!("Generated leaves:\n{leaves_rendered}");
    }

    debug!("Generating graph from leaves");

    let graph = match Graph::new(pats, config) {
        Ok(nfa) => nfa,
        Err(msg) => {
            let mut errors = Errors::default();
            errors.err(msg, item_span);
            return impl_logos(errors.render().unwrap());
        }
    };

    debug!("Generated Automaton:\n{:?}", graph.dfa());

    if cfg!(feature = "debug") {
        debug!("Generated Graph:\n{graph}");
    }

    if cfg!(feature = "debug") {
        if let Some(export_path) = parser.export_path.as_ref() {
            debug!("Exporting graphs");
            let lower_name = name.to_string().to_lowercase();

            if let Err(err) = generate_graphs(export_path, &lower_name, &graph) {
                debug!("Failed to export graphs: {err}");
            }
        }
    }

    debug!("Checking if any two tokens have the same priority");

    for DisambiguationError(matching) in graph.errors() {
        for leaf_id in &matching {
            let leaf = &graph.leaves()[leaf_id.0];
            let priority = leaf.priority;

            let matching = matching
                .iter()
                .filter(|&id| id != leaf_id)
                .map(|matchind_id| format!("  {}", &graph.leaves()[matchind_id.0]))
                .collect::<Vec<_>>()
                .join("\n");

            parser.err(
                format!(
                    concat!(
                        "The pattern {} can match simultaneously with the following variants:\n",
                        "{}\n",
                        "\n",
                        "(all at the priority {})"
                    ),
                    leaf, matching, priority
                ),
                leaf.span,
            );
        }
    }

    if let Some(errors) = parser.errors.render() {
        return impl_logos(errors);
    }

    debug!("Generating code from graph");

    let config = crate::generator::Config {
        use_state_machine_codegen: cfg!(feature = "state_machine_codegen"),
    };
    let mut generator = Generator::new(config, name, &this, &graph, &error_callback);

    let body = generator.generate();
    impl_logos(quote! {
        use #logos_path::internal::{
            LexerInternal,
            CallbackRetVal,
            CallbackResult,
            SkipRetVal,
            SkipResult,
        };
        use std::result::Result as _Result;
        use std::option::Option as _Option;
        use #logos_path::Logos;

        type _Lexer<'s> = #logos_path::Lexer<'s, #this>;

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

fn generate_graphs(path_str: &str, name: &str, graph: &Graph) -> Result<(), Box<dyn Error>> {
    let path = Path::new(path_str).to_owned();

    let (dot_path, mmd_path) = match path.extension().map(OsStr::to_str) {
        Some(Some("dot")) => (Some(path), None),
        Some(Some("mmd")) => (None, Some(path)),
        Some(_) => {
            return Err(String::from(
                "Export path must end in '.dot' or '.mmd', or it must be a directory.",
            )
            .into())
        }
        None => {
            let dot_path = path.join(format!("{name}.dot"));
            let mmd_path = path.join(format!("{name}.mmd"));
            (Some(dot_path), Some(mmd_path))
        }
    };

    for (path, is_dot) in [(dot_path, true), (mmd_path, false)] {
        let Some(path) = path else { continue };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let s = if is_dot {
            graph.get_dot()
        } else {
            graph.get_mermaid()
        }?;
        std::fs::write(path, s)?;
    }

    Ok(())
}
