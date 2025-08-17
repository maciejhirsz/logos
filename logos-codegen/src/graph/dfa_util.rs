use std::{collections::HashSet, iter};

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

pub fn iter_children<'a>(dfa: &'a OwnedDFA, state: StateID) -> impl Iterator<Item = StateID> + 'a {
    (0..=u8::MAX)
        .map(move |byte| dfa.next_state(state, byte))
        .chain(iter::once(dfa.next_eoi_state(state)))
}

/// This utility function returns every state accessible by the dfa
/// from a root state.
pub fn get_states<'a>(dfa: &'a OwnedDFA, root: StateID) -> impl Iterator<Item = StateID> {
    let mut states = HashSet::new();
    states.insert(root);
    let mut explore_stack = vec![root];
    while let Some(state) = explore_stack.pop() {
        for child in iter_children(dfa, state) {
            if states.insert(child) {
                explore_stack.push(child);
            }
        }
    }

    let mut sorted = states.into_iter().collect::<Vec<_>>();
    sorted.sort_unstable();
    sorted.into_iter()
}
