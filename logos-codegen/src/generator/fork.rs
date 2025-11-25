use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};

use crate::{
    generator::byte_to_tokens,
    graph::{Comparisons, State, StateData},
};

use super::Generator;

impl<'a> Generator<'a> {
    pub fn impl_fork(
        &mut self,
        state: State,
        state_data: &StateData,
        ignore_self: bool,
    ) -> TokenStream {
        if state_data.normal.len() > 2 {
            self.impl_fork_table(state, state_data, ignore_self)
        } else {
            self.impl_fork_match(state, state_data, ignore_self)
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
        if let &Some(eoi_state) = &state_data.eoi {
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

            let comparisons = byte_class.impl_with_cmp();
            let cmp_count: usize = comparisons.iter().map(|cmp| cmp.count_ops()).sum();

            let condition = if cmp_count > 2 {
                let (test_ident, test_mask) = self.add_test_to_lut(byte_class);
                quote! { #test_ident[byte as usize] & #test_mask != 0 }
            } else {
                let sub_conditions = comparisons
                    .into_iter()
                    .map(|cmp| {
                        let Comparisons { range, except } = cmp;
                        let start = byte_to_tokens(*range.start());
                        let end = byte_to_tokens(*range.end());
                        let exceptions = except
                            .into_iter()
                            .map(|ex| {
                                quote! { && byte != #ex }
                            })
                            .collect::<Vec<_>>();
                        if range.len() == 1 {
                            quote! { (byte == #start) }
                        } else {
                            quote! { (matches!(byte, #start ..= #end) #(#exceptions)*) }
                        }
                    })
                    .collect::<Vec<_>>();

                quote! { #(#sub_conditions) ||* }
            };
            let transition = self.state_transition(*next_state);
            inner_cases.append_all(quote! {
                if #condition {
                    offset += 1;
                    #transition
                }
            });
        }

        let eoi = self.fork_eoi(state, state_data);
        quote! {
            let other = lex.read::<u8>(offset);
            if let Some(byte) = other {
                #inner_cases
            } else {
                #eoi
            }
            _take_action!(lex, offset, context, state)
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
                    table[byte as usize] = Some(*next_state);
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
            let states_set = table.iter().filter_map(|&op| op).collect::<HashSet<_>>();
            let mut states = states_set.into_iter().collect::<Vec<_>>();
            // Sort for generated source stability
            states.sort_unstable();

            let idents = states
                .iter()
                .map(|state| &self.state_idents[state][1])
                .collect::<Vec<_>>();

            let actions = states
                .iter()
                .cloned()
                .map(|state| self.state_transition(state))
                .collect::<Vec<_>>();

            // Explicit fallthrough to otherwise case later in the function
            let match_body = quote! {
                #(LogosNextState::#idents => { #actions }),*
                LogosNextState::___ => {},
            };

            let table_elements = table
                .into_iter()
                .map(|state_op| match state_op {
                    Some(state) => self.state_idents[&state][1].to_token_stream(),
                    None => quote!(___),
                })
                .collect::<Vec<_>>();

            // Undo the unconditional increment if we don't have a next state
            quote! {
                #[derive(Copy, Clone)]
                enum LogosNextState {
                    ___,
                    #(#idents),*
                }
                const TABLE: [LogosNextState; 256] = {
                    use LogosNextState::*;
                    [ #(#table_elements),* ]
                };
                offset += 1;
                match TABLE[byte as usize] {
                    #match_body
                }
                offset -= 1;
            }
        };

        let eoi = self.fork_eoi(state, state_data);

        quote! {
            let other = lex.read::<u8>(offset);
            if let Some(byte) = other {
                #body
            } else {
                #eoi
            }
            _take_action!(lex, offset, context, state)
        }
    }
}
