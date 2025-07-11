use std::{collections::{hash_map::Entry, HashMap, VecDeque}, iter, ops::RangeInclusive};

use regex_automata::{dfa::{dense::DFA, Automaton, StartKind}, nfa::thompson::NFA, util::primitives::StateID, Anchored, MatchKind};

use crate::leaf::{Leaf, LeafId};

/// Disambiguation error during the attempt to merge two leaf
/// nodes with the same priority
#[derive(Clone, Debug)]
pub struct DisambiguationError(pub Vec<LeafId>);

type OwnedDFA = DFA<Vec<u32>>;

fn iter_matches<'a>(state_id: StateID, dfa: &'a OwnedDFA) -> impl Iterator<Item=LeafId> + 'a {
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

#[derive(Clone, Copy, Debug, Default)]
pub enum StateType {
    #[default]
    Normal,
    Accept(LeafId),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct State {
    pub dfa_id: StateID,
    pub context: Option<LeafId>,
}

impl State {
    fn filter_state_type(&self, state_type: StateType, graph: &Graph) -> StateType {
        if let StateType::Accept(accept_leaf_id) = state_type {
            if let Some(current_leaf_id) = self.context {
                let accept_prio = graph.leaves[accept_leaf_id.0].priority;
                let current_prio = graph.leaves[current_leaf_id.0].priority;
                if accept_prio < current_prio {
                    return StateType::Normal;
                }
            }
        }

        state_type
    }
}

#[derive(Debug, Default)]
pub struct StateData {
    pub state_type: StateType,
    pub normal: Vec<(ByteClass, State)>,
    pub eoi: Option<State>,
}

impl StateData {
    fn iter_children<'a>(&'a self) -> impl Iterator<Item=State> + 'a {
        self.normal.iter().map(|(_bc, s)| s.clone()).chain(self.eoi.iter().cloned())
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
pub struct Graph {
    leaves: Vec<Leaf>,
    dfa: OwnedDFA,
    edges: HashMap<State, StateData>,
    root: State,
    errors: Vec<DisambiguationError>,
}

#[derive(Debug)]
struct GraphTraverse {
    state_types: HashMap<StateID, StateType>,
    visit_stack: Vec<State>,
}

impl GraphTraverse {
    fn from_root(root: State) -> Self {
        Self { state_types: HashMap::new(), visit_stack: vec![root] }
    }

    fn get_state_type(&mut self, state_id: StateID, graph: &mut Graph) -> StateType {
        let vacant = match self.state_types.entry(state_id) {
            Entry::Occupied(occupied) => {
                return *occupied.get()
            },
            Entry::Vacant(vacant) => {
                vacant
            },
        };

        let matching_leaves = iter_matches(state_id, &graph.dfa).map(|leaf_id| (leaf_id, graph.leaves[leaf_id.0].priority)).collect::<Vec<_>>();

        let state_type = if let Some(&(highest_leaf_id, highest_priority)) = matching_leaves.iter().max_by_key(|(_leaf_id, priority)| priority) {
            let matching_prio_leaves: Vec<LeafId> = matching_leaves.into_iter().filter(|(leaf_id, priority)| *priority == highest_priority).map(|(leaf_id, _priority)| leaf_id).collect();
            if matching_prio_leaves.len() > 1 {
                graph.errors.push(DisambiguationError(matching_prio_leaves))
            }

            StateType::Accept(highest_leaf_id)
        } else {
            StateType::Normal
        };

        *vacant.insert(state_type)
    }
}

impl Graph {
    pub fn root(&self) -> State {
        self.root
    }

    pub fn get_states<'a>(&'a self) -> impl Iterator<Item=State> + 'a {
        self.edges.keys().cloned()
    }

    pub fn get_state_data(&self, state: &State) -> &StateData {
        self.edges.get(state).expect("Reached unreachable state")
    }

    pub fn leaves(&self) -> &Vec<Leaf> {
        &self.leaves
    }

    pub fn dfa(&self) -> &OwnedDFA {
        &self.dfa
    }

    pub fn errors<'b>(&'b self) -> impl Iterator<Item=DisambiguationError> + 'b {
        self.errors.iter().cloned()
    }

    pub fn new(leaves: Vec<Leaf>) -> Result<Self, String> {
        let hirs = leaves.iter().map(|leaf| leaf.pattern.hir()).collect::<Vec<_>>();

        // TODO: utf8 mode here
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


        let start_id = dfa.universal_start_state(Anchored::Yes)
            .expect("Lookaround assertions are disabled, so there should be a universal start state");
        let root = State { dfa_id: start_id, context: None };

        let mut graph = Self { leaves, dfa, edges: HashMap::new(), root, errors: Vec::new() };
        let mut traverse = GraphTraverse::from_root(root);

        while let Some(state) = traverse.visit_stack.pop() {
            if graph.edges.contains_key(&state) { continue }
            let state_data = graph.gen_state_data(state, &mut traverse);
            traverse.visit_stack.extend(state_data.iter_children());
            graph.edges.insert(state, state_data);
        }

        // TODO: prune nodes that don't lead to any more accept states before reaching the dead
        // node (0)

        Ok(graph)
    }

    fn gen_state_data(&mut self, state: State, traverse: &mut GraphTraverse) -> StateData {
        let state_type = state.filter_state_type(traverse.get_state_type(state.dfa_id, self), self);

        let mut result: HashMap<State, ByteClass> = HashMap::new();
        for input_byte in u8::MIN..=u8::MAX {
            let next_id = self.dfa.next_state(state.dfa_id, input_byte);

            // Don't need to account for the dead state
            if next_id.as_usize() == 0 { continue }

            let next_state = self.propagate_context(state, next_id, traverse);

            let bytes_to_next_state = result.entry(next_state).or_insert(ByteClass::new());
            bytes_to_next_state.add_byte(input_byte);
        }

        let normal = result.into_iter().map(|(s, bc)| (bc, s)).collect();

        let eoi_id  = self.dfa.next_eoi_state(state.dfa_id);
        let eoi = if eoi_id.as_usize() == 0 {
            None
        } else {
            Some(self.propagate_context(state, eoi_id, traverse))
        };

        StateData {
            state_type,
            normal,
            eoi,
        }
    }

    fn propagate_context(&mut self, prev: State, next_id: StateID, traverse: &mut GraphTraverse) -> State {
        let next_state_type = traverse.get_state_type(next_id, self);

        let context = match prev.filter_state_type(next_state_type, self) {
            StateType::Normal => prev.context,
            StateType::Accept(leaf_id) => Some(leaf_id),
        };

        State { dfa_id: next_id, context }
    }

}
