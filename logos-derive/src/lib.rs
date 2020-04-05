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
use syn::{Ident, Fields, ItemEnum, Attribute, GenericParam, Lifetime, Type};
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
    let mut mode = Mode::Utf8;
    let mut errors = Vec::new();
    let mut trivia = Some((true, Cow::borrowed(r"[ \t\f]"), Span::call_site()));

    let generics = match item.generics.params.len() {
        0 => {
            None
        },
        1 if matches!(item.generics.params.first(), Some(GenericParam::Lifetime(..))) => {
            Some(quote!(<'source>))
        },
        _ => {
            let span = item.generics.span();

            errors.push(Error::new("Logos currently supports permits a single lifetime generic.").span(span));

            None
        }
    };

    let mut parse_attr = |attr: &Attribute, errors: &mut Vec<_>| -> Result<(), error::SpannedError> {
        if let Some(ext) = util::value_from_attr("extras", attr)? {
            if let Some(_) = extras.replace(ext) {
                errors.push(Error::new("Only one #[extras] attribute can be declared.").span(super_span));
            }
        }

        if let Some(nested) = util::read_attr("logos", attr)? {
            if let Some(t) = util::value_from_nested::<Option<Literal>>("trivia", nested)? {
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

        Ok(())
    };

    for attr in &item.attrs {
        if let Err(err) = parse_attr(attr, &mut errors) {
            errors.push(err);
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
                errors.push(Error::new(
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
                    errors.push(Error::new(
                        format!(
                            "Logos currently only supports variants with one field, found {}",
                            fields.unnamed.len(),
                        )
                    ).span(fields.span()))
                }

                let mut field = fields.unnamed.first().expect("Already checked len; qed").ty.clone();

                // Replacing the lifetime should be done in the generator
                if let Type::Reference(ref mut typeref) = field {
                    let span = typeref.lifetime.as_ref().map(|lt| lt.span()).unwrap_or_else(|| Span::call_site());

                    typeref.lifetime = Some(Lifetime::new("'s", span));
                }

                Some(field)
            }
            Fields::Named(_) => {
                errors.push(Error::new("Logos doesn't support named fields yet.").span(span));

                None
            }
        };

        // Find if there is a callback defined before tackling individual declarations
        let global_callback = variant.attrs.iter()
            .find_map(|attr| {
                match util::value_from_attr::<Ident>("callback", attr) {
                    Ok(ident) => ident,
                    Err(err) => {
                        errors.push(err);
                        None
                    }
                }
            });

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
                errors.push(
                    Error::new(
                        "Since 0.11 Logos no longer requires the #[end] variant.\n\n\

                        For help with migration see release notes: https://github.com/maciejhirsz/logos/releases"
                    ).span(attr.span()));
            }

            let mut with_definition = |definition: Definition<Literal>| {
                let callback = definition.callback.or_else(|| global_callback.clone());
                let token = Leaf::token(variant).field(field.clone()).callback(callback);

                if let Literal::Bytes(..) = definition.value {
                    mode = Mode::Binary;
                }

                (token, definition.value)
            };

            match util::value_from_attr("token", attr) {
                Ok(Some(definition)) => {
                    let (token, value) = with_definition(definition);

                    let value = value.into_bytes();
                    let then = graph.push(token.priority(value.len()));

                    ropes.push(Rope::new(value, then));
                },
                Err(err) => errors.push(err),
                _ => (),
            }

            match util::value_from_attr("regex", attr) {
                Ok(Some(definition)) => {
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
                },
                Err(err) => errors.push(err),
                _ => (),
            }
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

    let extras = match extras {
        Some(ext) => quote!(#ext),
        None => quote!(()),
    };
    let source = match mode {
        Mode::Utf8 => quote!(str),
        Mode::Binary => quote!([u8]),
    };

    let error_def = match error {
        Some(error) => Some(quote!(const ERROR: Self = #name::#error;)),
        None => {
            errors.push(Error::new("missing #[error] token variant.").span(super_span));
            None
        },
    };

    let impl_logos = |body| {
        quote! {
            impl<'source> ::logos::Logos<'source> for #name #generics {
                type Extras = #extras;

                type Source = #source;

                const SIZE: usize = #size;

                #error_def

                fn lex(lex: &mut ::logos::Lexer<'source, Self>) {
                    #body
                }
            }
        }
    };

    if errors.len() > 0 {
        return impl_logos(quote! {
            fn _logos_derive_compile_errors() {
                #(#errors)*
            }
        }).into()
    }

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
    let tokens = impl_logos(quote! {
        use ::logos::internal::{LexerInternal, CallbackResult};

        type Lexer<'source> = ::logos::Lexer<'source, #name #generics>;

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
