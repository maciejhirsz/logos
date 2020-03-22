use crate::graph::{NodeId, Pattern};

#[cfg_attr(test, derive(PartialEq))]
pub struct Fork {
    /// Arms of the fork
    pub arms: Vec<Branch>,
    /// State to go to if no arms are matching
    pub miss: Option<NodeId>,
}

#[cfg_attr(test, derive(PartialEq))]
pub struct Branch {
    pub pattern: Pattern,
    pub then: NodeId,
}