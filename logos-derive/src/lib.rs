//! <p align="center">
//!      <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
//! </p>
//!
//! ## Create ridiculously fast Lexers.
//!
//! This is a `#[derive]` macro crate, [for documentation go to main crate](https://docs.rs/logos).

// The `quote!` macro requires deep recursion.
#![recursion_limit = "196"]

extern crate proc_macro;

mod util;
mod tree;
mod regex;
mod handlers;
mod generator;

use self::regex::Regex;
use self::tree::{Node, Fork, Leaf};
use self::util::{OptionExt, Definition, Literal, value_from_attr};
use self::handlers::{Handlers, Handler, Trivia};
use self::generator::Generator;

use quote::quote;
use proc_macro::TokenStream;
use syn::{ItemEnum, Fields, Ident};

enum Mode {
    Utf8,
    Binary,
}

#[proc_macro_derive(Logos, attributes(
    logos,
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

    let mut extras: Option<Ident> = None;
    let mut error = None;
    let mut end = None;
    let mut mode = Mode::Utf8;
    let mut trivia = Trivia::Default;

    // Initially we pack all variants into a single fork, this is where all the logic branching
    // magic happens.
    let mut fork = Fork::default();

    for attr in &item.attrs {
        if let Some(ext) = value_from_attr("extras", attr) {
            extras.insert(ext, |_| panic!("Only one #[extras] attribute can be declared."));
        }

        if let Some(nested) = util::read_attr("logos", attr) {
            for item in nested {
                if let Some(t) = util::value_from_nested::<Option<Literal>>("trivia", item) {
                    let (utf8, regex) = match t {
                        Some(Literal::Utf8(string)) => (true, string),
                        Some(Literal::Bytes(bytes)) => {
                            mode = Mode::Binary;

                            (false, util::bytes_to_regex_string(&bytes))
                        },
                        None => {
                            match trivia {
                                Trivia::Patterns(_) => {},
                                Trivia::Default => trivia = Trivia::Patterns(vec![]),
                            }

                            continue;
                        }
                    };

                    let node = Node::from_regex(&regex, utf8);

                    match node {
                        Node::Branch(ref branch) if branch.then.is_none() && branch.regex.len() == 1 => {
                            let pattern = branch.regex.first().clone();

                            match trivia {
                                Trivia::Patterns(ref mut patterns) => patterns.push(pattern),
                                Trivia::Default => trivia = Trivia::Patterns(vec![pattern]),
                            }

                            continue;
                        },
                        _ => {}
                    }

                    fork.insert(node.leaf(Leaf::Trivia));
                }
            }
        }
    }

    // Then the fork is split into handlers using all possible permutations of the first byte of
    // any branch as the index of a 256-entries-long table.
    let mut handlers = Handlers::new(trivia);

    // Finally the `Generator` will spit out Rust code for all the handlers.
    let mut generator = Generator::new(name);

    let mut variants = Vec::new();

    for variant in &item.variants {
        variants.push(&variant.ident);

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

            if let Some(definition) = value_from_attr::<Definition<Literal>>("token", attr) {
                let leaf = Leaf::Token {
                    token: variant,
                    callback: definition.callback,
                };

                let bytes = match definition.value {
                    Literal::Utf8(ref string) => string.as_bytes(),
                    Literal::Bytes(ref bytes) => {
                        mode = Mode::Binary;

                        &bytes
                    },
                };

                fork.insert(Node::new(Regex::sequence(bytes)).leaf(leaf));
            } else if let Some(definition) = value_from_attr::<Definition<Literal>>("regex", attr) {
                let leaf = Leaf::Token {
                    token: variant,
                    callback: definition.callback,
                };

                let (utf8, regex) = match definition.value {
                    Literal::Utf8(string) => (true, string),
                    Literal::Bytes(bytes) => {
                        mode = Mode::Binary;

                        (false, util::bytes_to_regex_string(&bytes))
                    },
                };

                fork.insert(Node::from_regex(&regex, utf8).leaf(leaf));
            }

            if let Some(callback) = value_from_attr("callback", attr) {
                generator.set_callback(variant, callback);
            }
        }
    }

    fork.pack();

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
        match handler {
            Handler::Error      => quote!(Some(_error)),
            Handler::Whitespace => quote!(None),
            Handler::Tree(tree) => generator.print_tree(tree),
        }
    }).collect::<Vec<_>>();

    let fns = generator.fns();

    let macro_lut =
        variants.iter()
            .enumerate()
            .map(|(index, _)| quote!( #name!(#index; $($x::$variant => $val;)* $def), ));

    let macro_matches =
        variants.iter()
            .enumerate()
            .map(|(index, variant)| quote!( (#index; #name::#variant => $val:expr; $( $rest:tt )* ) => ($val); ));

    let macro_shifts =
        variants.iter()
            .map(|variant| quote!( ($num:tt; #name::#variant => $val:expr; $( $rest:tt )* ) => (#name!($num; $($rest)*)); ));

    let source = match mode {
        Mode::Utf8   => quote!(Source),
        Mode::Binary => quote!(BinarySource),
    };

    let tokens = quote! {
        impl ::logos::Logos for #name {
            type Extras = #extras;

            const SIZE: usize = #size;
            const ERROR: Self = #name::#error;
            const END: Self = #name::#end;

            fn lexicon<'lexicon, 'source, Source>() -> &'lexicon ::logos::Lexicon<::logos::Lexer<Self, Source>>
            where
                Source: ::logos::Source<'source>,
                Self: ::logos::source::WithSource<Source>,
            {
                use ::logos::internal::LexerInternal;
                use ::logos::source::Split;

                type Lexer<S> = ::logos::Lexer<#name, S>;

                fn _error<'source, S: ::logos::Source<'source>>(lex: &mut Lexer<S>) {
                    lex.bump(1);

                    lex.token = #name::#error;
                }

                #fns

                &[#(#handlers),*]
            }
        }

        impl<'source, Source: ::logos::source::#source<'source>> ::logos::source::WithSource<Source> for #name {}

        #[macro_export]
        #[doc(hidden)]
        macro_rules! #name {
            // This pattern just handles trailing comma
            ($( $x:ident::$variant:ident => $val:expr, )+ _ => $def:expr,) => (
                #name!($( $x::$variant => $val, )* _ => $def)
            );

            // This pattern creates the actual LUT
            ($( $x:ident::$variant:ident => $val:expr, )+ _ => $def:expr) => (
                [
                    #( #macro_lut )*
                ]
            );

            // Patterns below match their variant to the exact index in the LUT
            #( #macro_matches )*

            // Variant not matching index recursively shifts the token stream
            // to the next variant in line
            #( #macro_shifts )*

            // Compile error for unknown variants
            ($num:tt; $x:ident::$var:ident => $val:expr; $( $rest:tt )*) => (
                compile_error!(concat!(stringify!($x), "::", stringify!($var), " is not a valid variant of ", stringify!(#name)))
            );

            // If the pattern above exhausted all possibilities, print default value
            ($num:expr; $def:expr) => ($def);
        }
    };

    // panic!("{}", tokens);

    TokenStream::from(tokens).into()
}
