use crate::graph::{Token, Branch, Fork, Node, NodeBody, Range};

impl<T> From<Fork> for NodeBody<T> {
    fn from(fork: Fork) -> Self {
        NodeBody::Fork(fork)
    }
}

impl<'a> From<Token<'a>> for NodeBody<Token<'a>> {
    fn from(leaf: Token<'a>) -> Self {
        NodeBody::Leaf(leaf)
    }
}

fn is_printable(byte: u8) -> bool {
    byte.is_ascii_punctuation() | byte.is_ascii_alphanumeric() | byte.is_ascii_whitespace()
}

/// We don't need debug impls in release builds
// #[cfg(test)]
mod debug {
    use super::*;
    use std::fmt::{self, Debug};


    impl Debug for Range {
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

    impl Debug for Branch {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?} ⇒ :{}", self.pattern, self.then)
        }
    }

    impl Debug for Fork {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut list = f.debug_list();

            struct Miss<T>(T);

            impl<T: fmt::Display> Debug for Miss<T> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "_ ⇒ :{}", self.0)
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

    impl<T: Debug> Debug for Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, ":{} ", self.id)?;

            self.body.fmt(f)
        }
    }

    impl<T: Debug> Debug for NodeBody<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                NodeBody::Fork(fork) => fork.fmt(f),
                NodeBody::Leaf(leaf) => leaf.fmt(f),
            }
        }
    }

    impl Debug for Token<'_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, ":T {}", self.ident)?;

            if let Some(ref callback) = self.callback {
                write!(f, " ({})", callback)?;
            }

            Ok(())
        }
    }

    use std::ops::RangeInclusive;

    impl From<RangeInclusive<u8>> for Range {
        fn from(range: RangeInclusive<u8>) -> Range {
            Range(*range.start(), *range.end())
        }
    }

    impl From<RangeInclusive<char>> for Range {
        fn from(range: RangeInclusive<char>) -> Range {
            Range(*range.start() as u8, *range.end() as u8)
        }
    }
}
