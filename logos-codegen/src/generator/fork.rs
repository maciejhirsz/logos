use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};

use crate::{
    generator::byte_to_tokens,
    graph::{State, StateData},
};

use super::Generator;

impl<'a> Generator<'a> {
    pub fn impl_fork(&mut self, state: State, state_data: &StateData) -> TokenStream {
        const ENABLE_TABLE: bool = true;
        const ENABLE_LOOP: bool = true;

        let self_edge = state_data
            .normal
            .iter()
            .filter(|(_bc, next_state)| next_state == &state)
            .collect::<Vec<_>>();
        assert!(self_edge.len() <= 1);

        let mut result = TokenStream::new();

        if ENABLE_LOOP {
            if let Some((bc, _)) = self_edge.first() {
                result.append_all(self.impl_fast_loop(bc));
            }
        }

        let fork = if ENABLE_TABLE && state_data.normal.len() > 2 {
            self.impl_fork_table(state, state_data, ENABLE_LOOP)
        } else {
            self.impl_fork_match(state, state_data, ENABLE_LOOP)
        };
        result.append_all(fork);

        result
    }

    /// Generate code for if state edge applies:
    ///  - return the current token (active context)
    ///  - or an error (no active context).
    fn fork_otherwise(&self, state: State) -> TokenStream {
        if let Some(leaf_id) = self.graph.get_state(state).context {
            self.generate_leaf(&self.graph.leaves()[leaf_id.0])
        } else {
            // if we reached eoi, we are already at the end of the input
            // so don't add 1 to offset.
            quote! {
                lex.end_to_boundary(offset + if other.is_some() { 1 } else { 0 });
                return Some(Err(_make_error(lex)));
            }
        }
    }

    // Generate code for encountering the end of input.
    // If we are not in the middle of a token, return None (the iterator is ended)
    // If the state has an EOI node, transition to it.
    // Otherwise, fall through to later code.
    fn fork_eoi(&self, state: State, state_data: &StateData) -> TokenStream {
        let mut eoi = TokenStream::new();
        if state == self.graph.root() {
            // If we just started lexing and are at the end of input, return None
            eoi.append_all(quote! { if lex.offset() == offset { return None } });
        }
        if let Some(eoi_state) = &state_data.eoi {
            let transition = self.state_transition(eoi_state);
            eoi.append_all(quote! {
                offset += 1;
                #transition
            });
        }

        eoi
    }

    fn impl_fork_match(
        &mut self,
        state: State,
        state_data: &StateData,
        ignore_self: bool,
    ) -> TokenStream {
        // Generate a match arm for each byte class, with each body being a state transition
        let mut inner_cases = TokenStream::new();
        for (byte_class, next_state) in &state_data.normal {
            if ignore_self && next_state == &state {
                continue;
            }

            let patterns = byte_class.ranges.iter().map(|range| {
                let start = byte_to_tokens(*range.start());
                let end = byte_to_tokens(*range.end());
                if range.len() == 1 {
                    quote! { Some(#start) }
                } else {
                    quote! { Some(#start ..= #end) }
                }
            });
            let transition = self.state_transition(next_state);
            inner_cases.append_all(quote! {
                #(#patterns)|* => {
                    offset += 1;
                    #transition
                },
            });
        }

        let eoi = self.fork_eoi(state, state_data);
        let otherwise = self.fork_otherwise(state);
        quote! {
            match lex.read::<u8>(offset) {
                #inner_cases
                other => {
                    if other.is_none() {
                        #eoi
                    }
                    #otherwise
                }
            }
        }
    }

    fn impl_fork_table(
        &mut self,
        state: State,
        state_data: &StateData,
        ignore_self: bool,
    ) -> TokenStream {
        // Generate a match arm for each byte class, with each body being a state transition
        let mut table = vec![None; 256];
        for (byte_class, next_state) in &state_data.normal {
            if ignore_self && next_state == &state {
                continue;
            }

            for range in &byte_class.ranges {
                for byte in range.clone() {
                    table[byte as usize] = Some(next_state);
                }
            }
        }

        // We need this distinction because the state machine states can be stored directly in the
        // table, while the function calls need a table of enums followed by a match to satisfy the
        // borrow checker (the state function signatures contain lifetimes).
        let body = if self.config.use_state_machine_codegen {
            let table_elements = table
                .into_iter()
                .map(|state_op| match state_op {
                    Some(state) => {
                        let val = self.state_value(state);
                        quote!(Some(#val))
                    }
                    None => quote!(None),
                })
                .collect::<Vec<_>>();

            let action = self.state_action(quote!(next_state));
            quote! {
                const TABLE: [_Option<LogosState>; 256] = [#(#table_elements),*];
                let next_state = TABLE[byte as usize];
                if let Some(next_state) = next_state {
                    offset += 1;
                    #action
                }
            }
        } else {
            let states_set = table
                .iter()
                .filter_map(|&op| op.cloned())
                .collect::<HashSet<_>>();
            let mut states = states_set.into_iter().collect::<Vec<_>>();
            // Sort for generated source stability
            states.sort_unstable();

            let mut match_body = TokenStream::new();
            for state in &states {
                let ident = self.get_ident(&state);
                let action = self.state_transition(&state);
                match_body.append_all(quote! {
                    Some(LogosNextState::#ident) => {
                        offset += 1;
                        #action
                    },
                });
            }
            // Explicit fallthrough to otherwise case later in the function
            match_body.append_all(quote! {
                None => {}
            });

            let state_idents = states
                .iter()
                .map(|state| self.get_ident(state))
                .collect::<Vec<_>>();
            let table_elements = table
                .into_iter()
                .map(|state_op| match state_op {
                    Some(state) => {
                        let val = self.state_value(state);
                        quote!(Some(LogosNextState::#val))
                    }
                    None => quote!(None),
                })
                .collect::<Vec<_>>();
            quote! {
                enum LogosNextState {
                    #(#state_idents),*
                }
                const TABLE: [_Option<LogosNextState>; 256] = [#(#table_elements),*];
                match TABLE[byte as usize] {
                    #match_body
                }
            }
        };

        let eoi = self.fork_eoi(state, state_data);
        let otherwise = self.fork_otherwise(state);

        // TODO: once we optimize more in the graph module, we might not need this anymore
        if state_data.normal.is_empty() {
            quote! {
                let other = lex.read::<u8>(offset);
                if other.is_none() {
                    #eoi
                }
                #otherwise
            }
        } else {
            quote! {
                let other = lex.read::<u8>(offset);
                if let Some(byte) = other {
                    #body
                } else {
                    #eoi
                }
                #otherwise
            }
        }
    }
}
