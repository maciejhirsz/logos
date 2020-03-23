use crate::graph::{Token, Rope, Fork, Node, NodeBody, Range};

impl<T> From<Fork> for NodeBody<T> {
    fn from(fork: Fork) -> Self {
        NodeBody::Fork(fork)
    }
}
impl<T> From<Rope> for NodeBody<T> {
    fn from(rope: Rope) -> Self {
        NodeBody::Rope(rope)
    }
}

impl From<Token> for NodeBody<Token> {
    fn from(leaf: Token) -> Self {
        NodeBody::Leaf(leaf)
    }
}

fn is_ascii(byte: u8) -> bool {
    (byte >= 0x20) & (byte < 0x7F)
}

/// We don't need debug impls in release builds
// #[cfg(test)]
mod debug {
    use super::*;
    use std::fmt::{self, Debug, Display};


    impl Debug for Range {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let Range(start, end) = *self;

            f.write_str("/")?;
            match is_ascii(start) {
                true => write!(f, "{}", start as char),
                false => write!(f, "{:02X}", start),
            }?;
            if start != end {
                match is_ascii(end) {
                    true => write!(f, "-{}", end as char),
                    false => write!(f, "-{:02X}", end),
                }?;
            }
            f.write_str("/")
        }
    }

    impl Display for Range {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            <Range as Debug>::fmt(self, f)
        }
    }

    struct Arm<T, U>(T, U);

    impl<T, U> Debug for Arm<T, U>
    where
        T: Display,
        U: Display,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{} ⇒ {}", self.0, self.1)
        }
    }

    impl Debug for Fork {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut list = f.debug_list();

            for (range, then) in self.branches() {
                list.entry(&Arm(range, then));
            }
            if let Some(id) = self.miss {
                list.entry(&Arm('_', id));
            }

            list.finish()
        }
    }

    impl Debug for Rope {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            use std::fmt::Write;

            let mut rope = '"'.to_string();
            rope.push_str(&String::from_utf8_lossy(&self.pattern));
            rope.push('"');

            match self.miss {
                Some(id) => {
                    let mut list = f.debug_list();

                    list.entry(&Arm(rope, self.then));
                    list.entry(&Arm('_', id));

                    list.finish()
                },
                None => Arm(rope, self.then).fmt(f),
            }
        }
    }

    impl Debug for Token {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "::{}", self.ident)?;

            if let Some(ref callback) = self.callback {
                write!(f, " ({})", callback)?;
            }

            Ok(())
        }
    }

    impl PartialEq for Fork {
        fn eq(&self, other: &Self) -> bool {
            self.miss == other.miss && self.branches().eq(other.branches())
        }
    }

    impl<T: Debug> Debug for Node<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{} ", self.id)?;

            self.body.fmt(f)
        }
    }

    impl<T: Debug> Debug for NodeBody<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                NodeBody::Fork(fork) => fork.fmt(f),
                NodeBody::Rope(rope) => rope.fmt(f),
                NodeBody::Leaf(leaf) => leaf.fmt(f),
            }
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

    impl<T> PartialEq<Rope> for NodeBody<T> {
        fn eq(&self, other: &Rope) -> bool {
            match self {
                NodeBody::Rope(rope) => rope == other,
                _ => false
            }
        }
    }

    impl<T> PartialEq<Fork> for NodeBody<T> {
        fn eq(&self, other: &Fork) -> bool {
            match self {
                NodeBody::Fork(fork) => fork == other,
                _ => false
            }
        }
    }
}
