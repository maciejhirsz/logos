use std::collections::HashSet;

use regex_automata::{
    dfa::{dense::DFA, Automaton},
    util::primitives::StateID,
};

use crate::leaf::LeafId;

pub type OwnedDFA = DFA<Vec<u32>>;

/// This utility implements an iterator over the matching patterns of a given dfa state
pub fn iter_matches<'a>(state_id: StateID, dfa: &'a OwnedDFA) -> impl Iterator<Item = LeafId> + 'a {
    let num_matches = if dfa.is_match_state(state_id) {
        dfa.match_len(state_id)
    } else {
        0
    };

    (0..num_matches).map(move |match_idx| {
        let pattern_id = dfa.match_pattern(state_id, match_idx);
        LeafId::from(pattern_id)
    })
}

/// This utility function returns every state accessible by the dfa
/// from a root state.
pub fn get_states<'a>(dfa: &'a OwnedDFA, root: StateID) -> impl Iterator<Item = StateID> {
    let mut states = HashSet::new();
    states.insert(root);
    let mut explore_stack = vec![root];
    while let Some(state) = explore_stack.pop() {
        for byte in 0..u8::MAX {
            let next_state = dfa.next_state(state, byte);
            if states.insert(next_state) {
                explore_stack.push(next_state)
            }
        }

        let eoi_state = dfa.next_eoi_state(state);
        if states.insert(eoi_state) {
            explore_stack.push(eoi_state)
        }
    }

    states.into_iter()
}
