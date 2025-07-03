use fnv::{FnvHashMap as Map, FnvHashSet as Set};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use regex_automata::dfa::dense::DFA;
use regex_automata::util::primitives::StateID;
use syn::Ident;

use crate::graph::Graph;
use crate::util::{MaybeVoid, ToIdent};

// mod context;
// mod fork;
// mod leaf;
// mod rope;
// mod tables;

// use self::context::Context;
// use self::tables::TableStack;

pub struct Generator<'a> {
    /// Name of the type we are implementing the `Logos` trait for
    name: &'a Ident,
    /// Name of the type with any generics it might need
    this: &'a TokenStream,
    /// Reference to the graph with all of the nodes
    graph: &'a Graph<'a>,
    /// Buffer with functions growing during generation
    rendered: TokenStream,
    /// Function name identifiers
    idents: Map<StateID, Ident>,

    // TODO
    // Identifiers for helper functions matching a byte to a given
    // set of ranges
    // tests: Map<Vec<Range>, Ident>,
    // Related to above, table stack manages tables that need to be
    // tables: TableStack,
}

impl<'a> Generator<'a> {
    pub fn new(
        name: &'a Ident,
        this: &'a TokenStream,
        graph: &'a Graph<'a>,
    ) -> Self {

        Generator {
            name,
            this,
            graph,
            rendered: TokenStream::new(),
            idents: Map::default(),
        }
    }

    pub fn generate(mut self) -> TokenStream {
        // let root = self.goto(self.root, Context::default()).clone();
        // let rendered = &self.rendered;
        // let tables = &self.tables;

        quote! {
        }
    }

    fn generate_fn(&mut self, id: StateID) {
        let body = MaybeVoid::Void;
        let ident = self.generate_ident(id);
        let out = quote! {
            #[inline]
            fn #ident<'s>(lex: &mut Lexer<'s>) {
                #body
            }
        };

        self.rendered.append_all(out);
    }

    fn generate_ident(&mut self, id: StateID) -> &Ident {
        self.idents.entry(id).or_insert_with(|| {
            format!("goto{}", id.as_usize()).to_ident()
        })
    }

}
