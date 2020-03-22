use crate::graph::{NodeId, Pattern};

#[cfg_attr(test, derive(PartialEq))]
pub struct Fork {
    /// Arms of the fork
    arms: Vec<Branch>,
    /// State to go to if no arms are matching
    miss: Option<NodeId>,
}

impl Fork {
    pub fn new<M: Into<Option<NodeId>>>(miss: M) -> Self {
        Fork {
            arms: Vec::new(),
            miss: miss.into(),
        }
    }

    pub fn arms(&self) -> &[Branch] {
        &self.arms
    }

    pub fn miss(&self) -> Option<NodeId> {
        self.miss
    }

    #[cfg(test)]
    pub fn arm(mut self, pattern: Pattern, then: NodeId) -> Self {
        self.arms.push(Branch { pattern, then });
        self
    }
}

#[cfg_attr(test, derive(PartialEq))]
pub struct Branch {
    pub pattern: Pattern,
    pub then: NodeId,
}