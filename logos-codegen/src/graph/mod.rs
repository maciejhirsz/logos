use std::ascii::escape_default;
use std::collections::HashSet;
use std::fmt;
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::RangeInclusive,
};

use dfa_util::{get_states, iter_matches, OwnedDFA};
use regex_automata::{
    dfa::{dense::DFA, Automaton, StartKind},
    nfa::thompson::NFA,
    util::primitives::StateID,
    Anchored, MatchKind,
};

use crate::leaf::{Leaf, LeafId};

mod export;
mod dfa_util;

/// A configuration used to construct a graph
#[derive(Debug)]
pub struct Config {
    /// When true, the graph should only allow matching valid UTF-8 sequences of bytes.
    pub utf8_mode: bool,
}

/// Disambiguation error when a DFA state matches
/// two (or more) leaves with the same priority
#[derive(Clone, Debug)]
pub struct DisambiguationError(pub Vec<LeafId>);

/// This type holds information about a given [State]. Namely, whether
/// it is a match state for a leaf or not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateType {
    pub accept: Option<LeafId>,
    pub early_accept: Option<LeafId>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct State(usize);

/// This type uniquely identifies the state of the Logos state machine.
/// Note that, in addition to the `regex-automata` DFA state, we also
/// keep track of whether a match has been encountered or not (In regex-automata,
/// this is accounted for at the regex engine level, above the DFA).

/// TODO
struct Todo;

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "state{}", self.0)
    }
}

/// This struct includes all information that should be attached to [State] but does not uniquely
/// identify State, which facilitates building a HashMap<State, StateData> structure.
#[derive(Clone, Debug, Default)]
pub struct StateData {
    /// The corresponding `regex_automata` state
    pub dfa_id: StateID,
    /// The most recently matched leaf (if any)
    pub context: Option<LeafId>,
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
    fn new(dfa_id: StateID) -> Self {
        Self {
            dfa_id,
            ..Default::default()
        }
    }

    fn with_context(dfa_id: StateID, context: Option<LeafId>) -> Self {
        Self {
            dfa_id,
            context,
            ..Default::default()
        }
    }

    /// An iterator over all [State] objects directly reachable from this state
    fn iter_children<'a>(&'a self) -> impl Iterator<Item = State> + 'a {
        self.normal
            .iter()
            .map(|(_bc, s)| s.clone())
            .chain(self.eoi.iter().cloned())
    }
}

impl fmt::Display for StateData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state_type {
            StateType {
                early_accept: Some(leaf_id),
                ..
            } => write!(f, "Early({})", leaf_id.0)?,
            StateType {
                accept: Some(leaf_id),
                ..
            } => write!(f, "Accept({})", leaf_id.0)?,
            StateType { .. } => write!(f, "Normal")?,
        }
        if f.alternate() {
            write!(f, " {{\n")?;
            for (bc, state) in &self.normal {
                write!(f, "  {} => {}\n", &bc.to_string(), state)?;
            }
            write!(f, "}}")?;
        }
        Ok(())
    }
}

/// This struct represents a subset of the possible bytes x00 through xFF
#[derive(Clone, Debug)]
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
    states: Vec<StateData>,
    /// The initial state (root) of the graph
    root: State,
    /// Any disambiguation errors encountered when constructing the graph
    errors: Vec<DisambiguationError>,
}

impl Graph {
    /// Get the root (initial) state of the graph
    pub fn root(&self) -> State {
        self.root
    }

    /// Iterate over all of the states of the graph
    pub fn iter_states<'a>(&'a self) -> impl Iterator<Item = State> + 'a {
        (0..self.states.len()).map(State)
    }

    /// Get a reference to the [StateData] corresponding to a state
    pub fn get_state(&self, state: State) -> &StateData {
        &self.states[state.0]
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
        let mut graph = Self::new_no_context(leaves, config)?;
        let mut ctx_traverse = ContextTraverse::new(graph.root, &graph);

        while let Some((no_ctx_state, ctx_state)) = ctx_traverse.next_state() {
            let orig_state_data = &graph.states[no_ctx_state.0];
            let context = ctx_traverse.ctx_states[ctx_state.0].context;

            let mut normal = Vec::new();
            for (bc, next_state) in orig_state_data.normal.iter().cloned() {
                let next_ctx_state = ctx_traverse.get_ctx_state(next_state, context);
                normal.push((bc, next_ctx_state));
            }
            ctx_traverse.ctx_states[ctx_state.0].normal = normal;

            ctx_traverse.ctx_states[ctx_state.0].eoi = orig_state_data.eoi.map(|eoi_state| {
                ctx_traverse.get_ctx_state(eoi_state, context)
            });

        }

        graph.states = ctx_traverse.ctx_states;
        graph.root = State(0);

        // Future optimizations:
        // TODO: prune nodes that don't lead to any more accept states before reaching the dead
        // node (0)
        //
        // TODO: implement early matching (so we don't need to read an extra byte) in cases where
        // all transitions lead to a match state for the same leaf

        Ok(graph)
    }

    fn new_no_context(leaves: Vec<Leaf>, config: Config) -> Result<Self, String> {
        let hirs = leaves
            .iter()
            .map(|leaf| leaf.pattern.hir())
            .collect::<Vec<_>>();

        let nfa_config = NFA::config().shrink(true).utf8(config.utf8_mode);
        let nfa = NFA::compiler()
            .configure(nfa_config)
            .build_many_from_hir(&hirs)
            .map_err(|err| {
                format!(
                    "Logos encountered an error compiling the NFA for this regex: {}",
                    err
                )
            })?;

        let dfa_config = DFA::config()
            .accelerate(false)
            .byte_classes(false)
            .minimize(true)
            .match_kind(MatchKind::All)
            .start_kind(StartKind::Anchored);
        let dfa = DFA::builder()
            .configure(dfa_config)
            .build_from_nfa(&nfa)
            .map_err(|err| {
                format!(
                    "Logos encountered an error compiling the DFA for this regex: {}",
                    err
                )
            })?;

        let Some(start_id) = dfa.universal_start_state(Anchored::Yes) else {
            return Err(concat!(
                "This Regex is missing a universal start state, which is unsupported by logos. ",
                "This is most likely do to a lookbehind assertion at the start of the regex."
            )
            .into());
        };
        if dfa.has_empty() {
            return Err(
                "This Regex may match an empty string, which is unsupported by logos.".into(),
            );
        }

        let mut states = Vec::new();
        let dfa_lookup = get_states(&dfa, start_id).enumerate().map(|(idx, dfa_id)| {
            states.push(StateData::new(dfa_id));
            (dfa_id, State(idx))
        }).collect::<HashMap<StateID, State>>();
        let root = dfa_lookup[&start_id];

        let mut errors = Vec::new();
        for state_data in &mut states {
            let dfa_id = state_data.dfa_id;
            match Self::get_state_type(dfa_id, &leaves, &dfa) {
                Ok(state_type) => state_data.state_type = state_type,
                Err(disambiguation_err) => errors.push(disambiguation_err),
            }
            let mut result: HashMap<State, ByteClass> = HashMap::new();
            for input_byte in u8::MIN..=u8::MAX {
                let next_id = dfa.next_state(dfa_id, input_byte);

                // Don't need to account for the dead state
                if next_id.as_usize() == 0 {
                    continue;
                }

                result.entry(dfa_lookup[&next_id]).or_insert(ByteClass::new()).add_byte(input_byte);
            }

            let mut normal: Vec<(ByteClass, State)> =
                result.into_iter().map(|(s, bc)| (bc, s)).collect();
            normal.sort_by_key(|(bc, _)| bc.ranges.first().map(|r| *r.start()));

            state_data.normal = normal;

            let eoi_id = dfa.next_eoi_state(dfa_id);
            state_data.eoi = if eoi_id.as_usize() == 0 {
                None
            } else {
                Some(dfa_lookup[&eoi_id])
            };
        }

        Ok(Graph {
            config,
            leaves,
            dfa,
            states,
            root,
            errors,
        })
    }

    /// Get the [StateType] of a [State] from the cache, or calculate it if it isn't present in the
    /// cache.
    fn get_state_type(state_id: StateID, leaves: &Vec<Leaf>, dfa: &OwnedDFA) -> Result<StateType, DisambiguationError> {
        // Get a list of all leaves that match in this state
        let matching_leaves = iter_matches(state_id, dfa)
            .map(|leaf_id| (leaf_id, leaves[leaf_id.0].priority))
            .collect::<Vec<_>>();

        // Find the highest priority that matches at this state
        if let Some(&(highest_leaf_id, highest_priority)) = matching_leaves
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
                return Err(DisambiguationError(matching_prio_leaves))
            }

            // TODO: Check for early accept here
            Ok(StateType {
                accept: Some(highest_leaf_id),
                early_accept: None,
            })
        } else {
            Ok(StateType::default())
        }
    }

    fn propagate_context(old_context: Option<LeafId>, new_kind: &StateType) -> Option<LeafId> {
        new_kind.accept.or(old_context)
    }

}

struct ContextTraverse<'a> {
    ctx_states: Vec::<StateData>,
    ctx_lookup: HashMap::<(State, Option<LeafId>), State>,
    // TODO: Replace this with an index
    ctx_stack: Vec::<(State, State)>,
    no_ctx_graph: &'a Graph,
}

impl<'a> ContextTraverse<'a> {
    fn new(no_ctx_root: State, no_ctx_graph: &'a Graph) -> Self {
        let ctx_root = State(0);
        let init_state = StateData::new(no_ctx_graph.states[no_ctx_root.0].dfa_id);
        Self {
            ctx_states: vec![init_state],
            ctx_lookup: [((no_ctx_root, None), ctx_root)].into_iter().collect(),
            ctx_stack: vec![(no_ctx_root, ctx_root)],
            no_ctx_graph,
        }
    }

    fn get_ctx_state(&mut self, no_ctx_next_state: State, current_context: Option<LeafId>) -> State {
        // TODO: BUG: Need to transfer accept (state_type) here
        let next_type = self.no_ctx_graph.states[no_ctx_next_state.0].state_type;
        let next_context = next_type.accept.or(current_context);
        match self.ctx_lookup.entry((no_ctx_next_state, next_context)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let index = self.ctx_states.len();
                let dfa_id = self.no_ctx_graph.get_state(no_ctx_next_state).dfa_id;
                let mut ctx_data = StateData::with_context(dfa_id, next_context);
                ctx_data.state_type = next_type;
                self.ctx_states.push(ctx_data);
                let ctx_state = *entry.insert(State(index));
                self.ctx_stack.push((no_ctx_next_state, ctx_state));
                ctx_state
            },
        }
    }

    fn next_state(&mut self) -> Option<(State, State)> {
        self.ctx_stack.pop()
    }
}
