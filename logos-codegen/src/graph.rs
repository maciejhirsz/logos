use std::{collections::{HashMap, VecDeque}, ops::RangeInclusive};

use regex_automata::{dfa::{dense::DFA, Automaton, StartKind}, nfa::thompson::NFA, util::primitives::StateID, Anchored, MatchKind};

use crate::leaf::{Leaf, LeafId, Leaves};

type OwnedDFA = DFA<Vec<u32>>;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct State {
    pub id: StateID,
    pub context: Option<LeafId>,
    // Techincally id/context should be the hash key and
    // is_accept should be part of the value with the edges,
    // but I'll worry about that later
    pub is_accept: bool,
}

impl State {
    fn from_prev(prev: Self, next_id: StateID, dfa: &OwnedDFA) -> Self {
        let mut next_state = State { id: next_id, context: prev.context, is_accept: false };
        if dfa.is_match_state(next_id) {
            for match_idx in 0..dfa.match_len(next_id) {
                let match_pattern = dfa.match_pattern(next_id, match_idx);
                if LeafId::from(match_pattern) >= next_state.context.unwrap_or(0.into()) {
                    next_state.is_accept = true;
                    next_state.context = Some(match_pattern.into());
                }
            }
        }

        next_state
    }
}

#[derive(Debug)]
pub struct ByteClass {
    pub ranges: Vec<RangeInclusive<u8>>,
}

impl ByteClass {
    fn new() -> Self {
        ByteClass { ranges: Vec::new() }
    }

    fn add_byte(&mut self, byte: u8) {
        if let Some(last) = self.ranges.last_mut() {
            if last.end() + 1 == byte {
                *last = *last.start()..=byte;
                return
            }
        }
        self.ranges.push(byte..=byte);
    }
}

#[derive(Debug)]
pub struct Transitions {
    pub normal: Vec<(ByteClass, State)>,
    pub eoi: Option<State>,
}

#[derive(Debug)]
pub struct Graph<'a> {
    leaves: Leaves<'a>,
    dfa: OwnedDFA,
    edges: HashMap<State, Transitions>,
    root: State,
}

impl<'a> Graph<'a> {
    pub fn try_new(leaves: Leaves<'a>) -> Result<Self, String> {
        let hirs = leaves.iter().map(|leaf| leaf.pattern.hir()).collect::<Vec<_>>();
        let config = NFA::config()
            .shrink(true);
        let nfa = NFA::compiler().configure(config).build_many_from_hir(&hirs).map_err(|err| format!("{}", err))?;
        if nfa.has_empty() {
            // TODO Better error handling
            return Err(String::from("Regex includes a zero length match"));
        }
        let config = DFA::config()
            .accelerate(false)
            .byte_classes(false)
            .minimize(true)
            .match_kind(MatchKind::All)
            .start_kind(StartKind::Anchored);
        let dfa = DFA::builder().configure(config).build_from_nfa(&nfa).map_err(|err| format!("{}", err))?;
        let mut edges = HashMap::new();
        let mut state_stack = Vec::new();
        let start_id = dfa.universal_start_state(Anchored::Yes).expect("Lookaround assertions are disabled, so there should be a universal start state");
        let root = State { id: start_id, context: None, is_accept: false };
        state_stack.push(root.clone());
        while let Some(state) = state_stack.pop() {
            if edges.contains_key(&state) { continue }
            let state_edges = Self::find_transitions(&dfa, state);
            state_stack.extend(state_edges.normal.iter().map(|(_bc, s)| s.clone()));
            if let Some(eoi) = state_edges.eoi {
                state_stack.push(eoi);
            }
            edges.insert(state, state_edges);
        }

        // TODO: prune nodes that don't lead to any more is_accept states before reaching the dead
        // node (0)

        Ok(Graph { leaves, dfa, edges, root })
    }

    pub fn root(&self) -> State {
        self.root
    }

    pub fn get_states(&'a self) -> impl Iterator<Item=State> + 'a {
        self.edges.keys().cloned()
    }

    pub fn get_transitions(&self, state: &State) -> &Transitions {
        self.edges.get(state).expect("Reached unreachable state")
    }

    fn find_transitions(dfa: &OwnedDFA, state: State) -> Transitions {
        let mut result: HashMap<State, ByteClass> = HashMap::new();
        for input_byte in u8::MIN..=u8::MAX {
            let next_id = dfa.next_state(state.id, input_byte);
            // Don't need to account for the dead state
            if next_id.as_usize() == 0 { continue }
            let next_state = State::from_prev(state, next_id, dfa);
            let edges = result.entry(next_state).or_insert(ByteClass::new());
            edges.add_byte(input_byte);
        }

        let normal = result.into_iter().map(|(s, bc)| (bc, s)).collect();
        let eoi = State::from_prev(state, dfa.next_eoi_state(state.id), dfa);
        let eoi = if eoi.id.as_usize() != 0 {
            Some(eoi)
        } else {
            None
        };

        Transitions { normal, eoi }
    }

    pub fn leaves(&self) -> &Leaves<'a> {
        &self.leaves
    }

    pub fn dfa(&self) -> &OwnedDFA {
        &self.dfa
    }
}
