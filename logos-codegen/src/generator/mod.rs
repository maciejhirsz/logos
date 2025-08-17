use std::collections::HashMap;

use fast_loop::fast_loop_macro;
use fnv::FnvHashMap as Map;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::graph::{ByteClass, Graph, State, StateType};
use crate::leaf::{Callback, InlineCallback};
use crate::util::ToIdent;

mod fast_loop;
mod fork;
mod leaf;

pub struct Config {
    pub use_state_machine_codegen: bool,
}

pub struct Generator<'a> {
    /// Configuration for the code generation
    config: Config,
    /// Name of the type we are implementing the `Logos` trait for
    name: &'a Ident,
    /// Name of the type with any generics it might need
    this: &'a TokenStream,
    /// Reference to the graph with all of the nodes
    graph: &'a Graph,
    /// Mapping of states to their identifiers.
    /// Identifiers are enum variants in the state machine codegen
    /// and function names in the tailcall codegen.
    idents: Map<State, Ident>,
    /// Callback for the default error type
    error_callback: &'a Option<Callback>,
    /// Bit masks that will be compressed into LUTs for fast looping
    loop_masks: HashMap<[bool; 256], usize>,
}

impl<'a> Generator<'a> {
    pub fn new(
        config: Config,
        name: &'a Ident,
        this: &'a TokenStream,
        graph: &'a Graph,
        error_callback: &'a Option<Callback>,
    ) -> Self {
        let idents = graph
            .iter_states()
            .map(|state| (state, state.to_string().to_ident()))
            .collect();

        Generator {
            config,
            name,
            this,
            graph,
            idents,
            error_callback,
            loop_masks: HashMap::new(),
        }
    }

    /// Generates the implementation (body) of the [Logos::lex] function
    pub fn generate(&mut self) -> TokenStream {
        let mut states = self.graph.iter_states().collect::<Vec<_>>();
        // Sort for repeatability (not dependent on hashmap iteration order)
        states.sort_unstable();
        let states_rendered = states
            .iter()
            .map(|&state| self.generate_state(state))
            .collect::<Vec<_>>();

        let init_state = &self.idents[&self.graph.root()];
        let mut all_idents = self.idents.values().collect::<Vec<_>>();
        // Sort for repeatability (not dependent on hashmap iteration order)
        all_idents.sort_unstable();

        let error_cb = self.generate_error_cb();
        let fast_loop_macro = fast_loop_macro(8);
        let loop_luts = self.render_luts();

        if self.config.use_state_machine_codegen {
            quote! {
                #fast_loop_macro
                #loop_luts
                #error_cb
                #[derive(Clone, Copy)]
                enum LogosState {
                    #(#all_idents),*
                }
                let mut state = LogosState::#init_state;
                let mut offset = lex.offset();
                loop {
                    match state {
                        #(#states_rendered)*
                    }
                }
            }
        } else {
            quote! {
                #fast_loop_macro
                #loop_luts
                #error_cb
                #(#states_rendered)*
                #init_state(lex, lex.offset())
            }
        }
    }

    fn get_ident(&self, state: &State) -> &Ident {
        self.idents.get(state).expect("Unreachable state found")
    }

    // Generates the definition for the `_make_error` function. This can be
    // specified using the `callback` argument of the `error` attribute.
    // Otherwise, it defaults to the `Default::default()`value.
    fn generate_error_cb(&self) -> TokenStream {
        let this = self.this;

        let body = match self.error_callback {
            Some(Callback::Label(label)) => quote! {
                let error = #label(lex);
                error.into()
            },
            Some(Callback::Inline(InlineCallback { arg, body, .. })) => quote! {
                let #arg = lex;
                let error = { #body };
                error.into()
            },
            None => quote! {
                <#this as Logos<'s>>::Error::default()
            },
        };

        quote! {
            #[inline]
            fn _make_error<'s>(lex: &mut _Lexer<'s>) -> <#this as Logos<'s>>::Error {
                #body
            }
        }
    }

    /// Generates the code to transition to a state.
    fn state_transition(&self, state: &State) -> TokenStream {
        return self.state_action(self.state_value(state));
    }

    /// Generates the code to transition to a state stored in an identifier
    fn state_action(&self, state_ident: TokenStream) -> TokenStream {
        match self.config.use_state_machine_codegen {
            true => quote! { state = #state_ident; continue; },
            false => quote! { return #state_ident(lex, offset); },
        }
    }

    /// Generates the code to quote a state's representation
    fn state_value(&self, state: &State) -> TokenStream {
        let state_ident = self.get_ident(&state);
        match self.config.use_state_machine_codegen {
            true => quote!(LogosState::#state_ident),
            false => quote!(#state_ident),
        }
    }

    /// Generates the body of a state. This is a match statement over
    /// the next byte, which determines the next state.
    ///
    /// It also instantiates the relevant leaf, if `state` has a context.
    ///
    /// In state machine codegen, the body is wrapped in a match arm for the
    /// `state`'s variant. In tailcall codegen, the body is inside of
    /// `state`'s function.
    fn generate_state(&mut self, state: State) -> TokenStream {
        let state_data = self.graph.get_state(state);

        // If we are in a match state, update the current token to
        // end at the current offset - 1.
        // The 1 comes from the 1 byte delayed match behavior
        // of the regex-automata crate.
        // TODO: this needs to be after the fast loop
        let setup = match state_data.state_type {
            StateType { early: Some(_), .. } => Some(quote! { lex.end(offset); }),
            StateType {
                accept: Some(_), ..
            } => Some(quote! { lex.end(offset - 1); }),
            StateType { .. } => None,
        };

        let fast_loop = self.maybe_impl_fast_loop(state);
        let fork = self.impl_fork(state, state_data, true);

        // Wrap body in a match arm or function depending on the current codegen
        let this_ident = self.get_ident(&state);
        if self.config.use_state_machine_codegen {
            quote! {
                LogosState::#this_ident => {
                    #fast_loop
                    #setup
                    #fork
                }
            }
        } else {
            let this = self.this;
            quote! {
                fn #this_ident<'s>(lex: &mut _Lexer<'s>, mut offset: usize)
                    -> _Option<_Result<#this, <#this as Logos<'s>>::Error>> {
                    #fast_loop
                    #setup
                    #fork
                }
            }
        }
    }

    fn table_ident(index: usize) -> Ident {
        format!("_TABLE_{index}").to_ident()
    }

    fn add_test_to_lut(&mut self, edge: &ByteClass) -> (Ident, u8) {
        let mut table_bits = [false; 256];
        for range in edge.ranges.iter() {
            for byte in range.clone() {
                table_bits[byte as usize] = true;
            }
        }

        let loop_id = if let Some(&existing) = self.loop_masks.get(&table_bits) {
            existing
        } else {
            let loop_id = self.loop_masks.len();
            self.loop_masks.insert(table_bits, loop_id);
            loop_id
        };

        let loop_table = loop_id / 8;
        let ident = Self::table_ident(loop_table);
        let loop_mask = 1u8 << (loop_id % 8);

        (ident, loop_mask)
    }

    pub fn render_luts(&self) -> TokenStream {
        let mut sorted = self.loop_masks.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|(_bits, id)| **id);
        let decls = sorted.chunks(8).enumerate().map(|(lut_idx, bit_arrs)| {
            let mut byte_arr = [0u8; 256];
            for (bit_index, (bits, _id)) in bit_arrs.iter().enumerate() {
                for (arr_idx, &bit) in bits.iter().enumerate() {
                    if bit {
                        byte_arr[arr_idx] |= 1 << bit_index;
                    }
                }
            }

            let ident = Self::table_ident(lut_idx);
            quote! { const #ident: [u8; 256] = [#(#byte_arr),*]; }
        });

        quote! { #(#decls)* }
    }
}

macro_rules! match_quote {
    ($source:expr; $($byte:tt,)* ) => {match $source {
        $( $byte => quote!($byte), )*
        byte => quote!(#byte),
    }}
}

/// Converts a byte to a byte literal that can be used to match it
fn byte_to_tokens(byte: u8) -> TokenStream {
    match_quote! {
        byte;
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
        b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j',
        b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't',
        b'u', b'v', b'w', b'x', b'y', b'z',
        b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J',
        b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T',
        b'U', b'V', b'W', b'X', b'Y', b'Z',
        b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',
        b'{', b'}', b'[', b']', b'<', b'>', b'-', b'=', b'_', b'+',
        b':', b';', b',', b'.', b'/', b'?', b'|', b'"', b'\'', b'\\',
    }
}
