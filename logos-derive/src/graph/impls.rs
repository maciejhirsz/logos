use std::fmt;

use super::{Leaf, Branch, Fork, Node, NodeBody};

impl<'a> Default for Leaf<'a> {
    fn default() -> Self {
        Leaf::Trivia
    }
}

impl<T> From<NodeBody<T>> for Node<T> {
    fn from(body: NodeBody<T>) -> Self {
        Node::new(body)
    }
}

impl<T> From<Fork> for NodeBody<T> {
    fn from(fork: Fork) -> Self {
        NodeBody::Fork(fork)
    }
}

impl<'a> From<Leaf<'a>> for NodeBody<Leaf<'a>> {
    fn from(leaf: Leaf<'a>) -> Self {
        NodeBody::Leaf(leaf)
    }
}

// #[cfg(test)]
// impl<T> PartialEq for Node<T>
// where
//     T: PartialEq,
// {
//     fn eq(&self, other: &Self) -> bool {
//         self.body == other.body
//     }
// }

// impl fmt::Debug for Leaf<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             Leaf::Token { token, callback } => {
//                 write!(f, "{}", token)?;

//                 if let Some(ref callback) = callback {
//                     write!(f, " ({})", callback)?;
//                 }
//             }
//             Leaf::Trivia => write!(f, "TRIVIA")?,
//         }

//         Ok(())
//     }
// }

// impl fmt::Debug for Branch {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         self.regex.fmt(f)?;

//         if let Some(ref then) = self.then {
//             f.write_str(" -> ")?;
//             then.fmt(f)?;
//         }

//         Ok(())
//     }
// }

// impl From<Branch> for Node<'_> {
//  fn from(branch: Branch) -> Self {
//      Node::Branch(branch)
//  }
// }

// impl<'a> From<Leaf<'a>> for Node<'a> {
//  fn from(leaf: Leaf<'a>) -> Self {
//      Node::Leaf(leaf)
//  }
// }