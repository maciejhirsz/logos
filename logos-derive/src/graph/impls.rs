use std::fmt;

use super::{Leaf, Branch, Fork, Node, NodeBody, Range};

impl<'a> Default for Leaf<'a> {
    fn default() -> Self {
        Leaf::Trivia
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

fn is_printable(byte: u8) -> bool {
    byte.is_ascii_punctuation() | byte.is_ascii_alphanumeric() | byte.is_ascii_whitespace()
}

#[cfg(test)]
mod debug {
    use super::*;

    impl fmt::Debug for Range {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let Range(start, end) = *self;

            match is_printable(start) {
                true => write!(f, "{}", start as char),
                false => write!(f, "{:02X}", start),
            }?;
            if start != end {
                match is_printable(end) {
                    true => write!(f, "-{}", end as char),
                    false => write!(f, "-{:02X}", end),
                }?;
            }
            Ok(())
        }
    }

    impl fmt::Debug for Branch {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?} ⇒ :{}", self.pattern, self.then)
        }
    }

    impl fmt::Debug for Fork {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut list = f.debug_list();

            struct Miss<T>(T);

            impl<T: fmt::Display> fmt::Debug for Miss<T> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "⤷ :{}", self.0)
                }
            }

            for arm in self.arms.iter() {
                list.entry(arm);
            }

            match self.miss {
                Some(id) => list.entry(&Miss(id)),
                None => list.entry(&Miss("ERR")),
            };

            list.finish()
        }
    }

    impl<T: fmt::Debug> fmt::Debug for Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, ":{} ", self.id)?;

            self.body.fmt(f)
        }
    }

    impl<T: fmt::Debug> fmt::Debug for NodeBody<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                NodeBody::Fork(fork) => fork.fmt(f),
                NodeBody::Leaf(leaf) => leaf.fmt(f),
            }
        }
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