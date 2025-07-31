use std::ascii::escape_default;
use std::fmt;
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::RangeInclusive,
};

use regex_automata::{
    dfa::{dense::DFA, Automaton, StartKind},
    nfa::thompson::NFA,
    util::primitives::StateID,
    Anchored, MatchKind,
};

use crate::leaf::{Leaf, LeafId};

mod export;

/// A configuration used to construct a graph
#[derive(Debug)]
pub struct Config {
    /// When true, leaf priority is more important than match length.
    /// When false, leaf priority is less important than match length.
    /// The less important metric is only used in the case of ties in the more important metric.
    pub prio_over_length: bool,
    /// When true, the graph should only allow matching valid UTF-8 sequences of bytes.
    pub utf8_mode: bool,
}

/// Disambiguation error when a DFA state matches
/// two (or more) leaves with the same priority
#[derive(Clone, Debug)]
pub struct DisambiguationError(pub Vec<LeafId>);

type OwnedDFA = DFA<Vec<u32>>;

/// This utility implements an iterator over the matching patterns of a given dfa state
fn iter_matches<'a>(state_id: StateID, dfa: &'a OwnedDFA) -> impl Iterator<Item = LeafId> + 'a {
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

/// This type holds information about a given [State]. Namely, whether
/// it is a match state for a leaf or not.
#[derive(Clone, Copy, Debug, Default)]
pub enum StateType {
    #[default]
    Normal,
    Accept(LeafId),
}

/// This type uniquely identifies the state of the Logos state machine.
/// Note that, in addition to the `regex-automata` DFA state, we also
/// keep track of whether a match has been encountered or not (In regex-automata,
/// this is accounted for at the regex engine level, above the DFA).
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct State {
    /// The corresponding `regex_automata` state
    pub dfa_id: StateID,
    /// The most recently matched leaf (if any)
    pub context: Option<LeafId>,
}

impl State {
    /// This function is used to "filter" the [StateType] that should be associated
    /// with this state. In most cases, it passes `state_type` through unchanged. However, in the
    /// case that `state_type` is a [StateType::Accept] and [State.context] is Some, and the
    /// `context` leaf is a higher priority, then the returned value is instead
    /// [StateType::Normal].
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

/// This struct includes all information that should be attached to [State] but does not uniquely
/// identify State, which facilitates building a HashMap<State, StateData> structure.
#[derive(Debug, Default)]
pub struct StateData {
    /// The type of the [State] object this struct defines
    pub state_type: StateType,
    /// The "normal" transitions (those that consume a byte of input) from this state to another
    /// state
    pub normal: Vec<(ByteClass, State)>,
    /// The "eoi" transition (the transition taken if this state immediately preceeds the end of
    /// the input), if any.
    pub eoi: Option<State>,
}

impl StateData {
    /// An iterator over all [State] objects directly reachable from this state
    fn iter_children<'a>(&'a self) -> impl Iterator<Item = State> + 'a {
        self.normal
            .iter()
            .map(|(_bc, s)| s.clone())
            .chain(self.eoi.iter().cloned())
    }
}

/// This struct represents a subset of the possible bytes x00 through xFF
#[derive(Debug)]
pub struct ByteClass {
    pub ranges: Vec<RangeInclusive<u8>>,
}

impl ByteClass {
    fn new() -> Self {
        ByteClass { ranges: Vec::new() }
    }

    /// Add the `byte` to the set of bytes that are included in this class
    fn add_byte(&mut self, byte: u8) {
        if let Some(last) = self.ranges.last_mut() {
            if last.end() + 1 == byte {
                *last = *last.start()..=byte;
                return;
            }
        }
        self.ranges.push(byte..=byte);
    }
}

impl fmt::Display for ByteClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, range) in self.ranges.iter().enumerate() {
            if range.start() == range.end() {
                write!(f, "{}", escape_default(*range.start()))?;
            } else {
                write!(
                    f,
                    "{}..={}",
                    escape_default(*range.start()),
                    escape_default(*range.end())
                )?;
            }

            if idx < self.ranges.len() - 1 {
                write!(f, "|")?;
            }
        }

        Ok(())
    }
}

/// This struct represents a complete state machine graph. The semantic are as follows.
///
/// Execution starts in the state indicated by the `root` field. To transition to a new state, the
/// executor reads a byte from the input, and then proceeds to a new state according to the current
/// states transitions (taking the EOI transition if there are no more bytes to read). Whenever the
/// executor reaches a state of the type [StateType::Accept], it should save the current offset - 1
/// into the input. When the executor reads an input byte (or EOI) that has no corresponding
/// transition, it should return a match on the leaf indicated by its context, using the span of
/// the input from where it began the match state to the saved offset.
#[derive(Debug)]
pub struct Graph {
    /// The config used to construct the graph
    config: Config,
    /// The leaves used to construct the graph
    leaves: Vec<Leaf>,
    /// The dfa used to construct the graph
    dfa: OwnedDFA,
    /// The states (and edges, within [StateData]), that make up the graph
    edges: HashMap<State, StateData>,
    /// The initial state (root) of the graph
    root: State,
    /// Any disambiguation errors encountered when constructing the graph
    errors: Vec<DisambiguationError>,
}

/// This struct holds information needed to traverse the state graph of the
/// [regex_automata::dfa::dense::DFA] efficiently.
#[derive(Debug)]
struct GraphTraverse {
    /// This is a cache of the [StateType] corresponding to each `regex_automata`'s [StateID], so
    /// that it only needs to be calculated once for each.
    state_types: HashMap<StateID, StateType>,
    /// This is a stack of [State]s that still need to be visited.
    visit_stack: Vec<State>,
}

impl GraphTraverse {
    fn from_root(root: State) -> Self {
        Self {
            state_types: HashMap::new(),
            visit_stack: vec![root],
        }
    }

    /// Get the [StateType] of a [State] from the cache, or calculate it if it isn't present in the
    /// cache.
    fn get_state_type(&mut self, state_id: StateID, graph: &mut Graph) -> StateType {
        let vacant = match self.state_types.entry(state_id) {
            Entry::Occupied(occupied) => return *occupied.get(),
            Entry::Vacant(vacant) => vacant,
        };

        // Get a list of all leaves that match in this state
        let matching_leaves = iter_matches(state_id, &graph.dfa)
            .map(|leaf_id| (leaf_id, graph.leaves[leaf_id.0].priority))
            .collect::<Vec<_>>();

        // Find the highest priority that matches at this state
        let state_type = if let Some(&(highest_leaf_id, highest_priority)) = matching_leaves
            .iter()
            .max_by_key(|(_leaf_id, priority)| priority)
        {
            // Find all the leaves that match at said highest priority
            let matching_prio_leaves: Vec<LeafId> = matching_leaves
                .into_iter()
                .filter(|(_leaf_id, priority)| *priority == highest_priority)
                .map(|(leaf_id, _priority)| leaf_id)
                .collect();
            // Ensure that only one leaf matches at said highest priority
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
    /// Get the root (initial) state of the graph
    pub fn root(&self) -> State {
        self.root
    }

    /// Iterate over all of the states of the graph
    pub fn get_states<'a>(&'a self) -> impl Iterator<Item = State> + 'a {
        self.edges.keys().cloned()
    }

    /// Get a reference to the [StateData] corresponding to a state
    pub fn get_state_data(&self, state: &State) -> &StateData {
        self.edges.get(state).expect("Reached unreachable state")
    }

    /// Get a reference to the leaves used to generate this graph
    pub fn leaves(&self) -> &Vec<Leaf> {
        &self.leaves
    }

    /// Get a reference to the DFA used to generate this graph
    pub fn dfa(&self) -> &OwnedDFA {
        &self.dfa
    }

    /// Iterate over all the disambiguation errors encountered while generating this graph
    pub fn errors<'b>(&'b self) -> impl Iterator<Item = DisambiguationError> + 'b {
        self.errors.iter().cloned()
    }

    /// Create a new graph using a given list of [Leaf] objects to match on and a [Config]
    pub fn new(leaves: Vec<Leaf>, config: Config) -> Result<Self, String> {
        let hirs = leaves
            .iter()
            .map(|leaf| leaf.pattern.hir())
            .collect::<Vec<_>>();

        let nfa_config = NFA::config().shrink(true).utf8(config.utf8_mode);
        let nfa = NFA::compiler()
            .configure(nfa_config)
            .build_many_from_hir(&hirs)
            .map_err(|err| format!("{}", err))?;
        if nfa.has_empty() {
            // TODO Better error handling
            return Err(String::from("Regex includes a zero length match"));
        }
        let dfa_config = DFA::config()
            .accelerate(false)
            .byte_classes(false)
            .minimize(true)
            .match_kind(MatchKind::All)
            .start_kind(StartKind::Anchored);
        let dfa = DFA::builder()
            .configure(dfa_config)
            .build_from_nfa(&nfa)
            .map_err(|err| format!("{}", err))?;

        let start_id = dfa.universal_start_state(Anchored::Yes).expect(
            // TODO: clearer error message here, because we didn't explicitly disable lookbehind
            // assertions
            "Lookaround assertions are disabled, so there should be a universal start state",
        );
        let root = State {
            dfa_id: start_id,
            context: None,
        };

        let mut graph = Self {
            leaves,
            dfa,
            edges: HashMap::new(),
            root,
            errors: Vec::new(),
            config,
        };

        // Now that we have created the DFA, we traverse all its states to build the graph from it
        let mut traverse = GraphTraverse::from_root(root);

        while let Some(state) = traverse.visit_stack.pop() {
            if graph.edges.contains_key(&state) {
                continue;
            }
            let state_data = graph.gen_state_data(state, &mut traverse);
            traverse.visit_stack.extend(state_data.iter_children());
            graph.edges.insert(state, state_data);
        }

        // TODO: prune nodes that don't lead to any more accept states before reaching the dead
        // node (0)
        //
        // TODO: implement early matching (so we don't need to read an extra byte) in cases where
        // all transitions lead to a match state for the same leaf

        Ok(graph)
    }

    /// For a given [State], create its corresponding [StateData], adding any newly encountered
    /// states to the `traverse` visit_stack.
    fn gen_state_data(&mut self, state: State, traverse: &mut GraphTraverse) -> StateData {
        let state_type = state.filter_state_type(traverse.get_state_type(state.dfa_id, self), self);

        let mut result: HashMap<State, ByteClass> = HashMap::new();
        for input_byte in u8::MIN..=u8::MAX {
            let next_id = self.dfa.next_state(state.dfa_id, input_byte);

            // Don't need to account for the dead state
            if next_id.as_usize() == 0 {
                continue;
            }

            let next_state = self.propagate_context(state, next_id, traverse);

            let bytes_to_next_state = result.entry(next_state).or_insert(ByteClass::new());
            bytes_to_next_state.add_byte(input_byte);
        }

        let mut normal: Vec<(ByteClass, State)> =
            result.into_iter().map(|(s, bc)| (bc, s)).collect();
        normal.sort_by_key(|(bc, _)| bc.ranges.first().map(|r| *r.start()));

        let eoi_id = self.dfa.next_eoi_state(state.dfa_id);
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

    /// Given a previous [State] (`prev`) and the next state's DFA id (`next_id`),
    /// create a new [State] for the DFA id by propogating the context from the previous state, or
    /// overwriting it if this new state matches a leaf.
    fn propagate_context(
        &mut self,
        prev: State,
        next_id: StateID,
        traverse: &mut GraphTraverse,
    ) -> State {
        let mut next_state_type = traverse.get_state_type(next_id, self);

        if self.config.prio_over_length {
            next_state_type = prev.filter_state_type(next_state_type, self);
        };

        let context = match next_state_type {
            StateType::Normal => prev.context,
            StateType::Accept(leaf_id) => Some(leaf_id),
        };

        State {
            dfa_id: next_id,
            context,
        }
    }
}
