use regex_automata::{dfa::dense::DFA, nfa::thompson::NFA};

use crate::leaf::{Leaf, Leaves};

pub struct Graph<'a> {
    leaves: Vec<Leaf<'a>>,
    dfa: DFA<Vec<u32>>,
    // TODO meta information
}

impl<'a> Graph<'a> {
    pub fn try_new(leaves: Leaves<'a>) -> Result<Self, String> {
        let leaves: Vec<Leaf> = leaves.into();
        let hirs = leaves.iter().map(|leaf| leaf.pattern.hir()).collect::<Vec<_>>();
        let nfa = NFA::compiler().build_many_from_hir(&hirs).map_err(|err| format!("{}", err))?;
        let dfa = DFA::builder().build_from_nfa(&nfa).map_err(|err| format!("{}", err))?;
        Ok(Graph { leaves, dfa })
    }
}
