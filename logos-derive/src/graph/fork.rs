use std::convert::TryInto;

use crate::graph::{NodeId, Range};

pub struct Fork {
    /// State to go to if no arms are matching
    miss: Option<NodeId>,
    /// LUT matching byte -> arm, using u8s as we can't have more arms than bytes
    lut: Box<[Option<NodeId>; 256]>,
}

impl Fork {
    pub fn new<M: Into<Option<NodeId>>>(miss: M) -> Self {
        Fork {
            miss: miss.into(),
            lut: Box::new([None; 256]),
        }
    }

    pub fn add_branch<R>(&mut self, range: R, then: NodeId)
    where
        R: Into<Range>,
    {
        for byte in range.into() {
            match &mut self.lut[byte as usize] {
                Some(other) if *other != then => {
                    unimplemented!()
                },
                opt => *opt = Some(then),
            }
        }
    }

    pub fn branches(&self) -> ForkIter<'_> {
        ForkIter {
            offset: 0,
            lut: &*self.lut,
        }
    }

    pub fn miss(&self) -> Option<NodeId> {
        self.miss
    }

    pub fn branch<R>(mut self, range: R, then: NodeId) -> Self
    where
        R: Into<Range>,
    {
        self.add_branch(range, then);
        self
    }
}

pub struct ForkIter<'a> {
    offset: usize,
    lut: &'a [Option<NodeId>; 256],
}

impl<'a> Iterator for ForkIter<'a> {
    type Item = (Range, NodeId);

    fn next(&mut self) -> Option<Self::Item> {
        // Consume empty slots
        self.offset += self.lut[self.offset..]
            .iter()
            .take_while(|next| next.is_none())
            .count();

        let then = self.lut.get(self.offset).copied().flatten()?;
        let start = self.offset;

        // Consume all slots with same NodeId target
        self.offset += self.lut[self.offset..]
            .iter()
            .take_while(|next| **next == Some(then))
            .count();

        Some((Range(start as u8, (self.offset - 1) as u8), then))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fork_iter() {
        let mut buf = [None; 256];

        for byte in b'4'..=b'7' {
            buf[byte as usize] = Some(1);
        }
        for byte in b'a'..=b'd' {
            buf[byte as usize] = Some(2);
        }

        let iter = ForkIter {
            offset: 0,
            lut: &buf,
        };

        assert_eq!(
            &[
                (Range(b'4', b'7'), 1),
                (Range(b'a', b'd'), 2),
            ],
            &*iter.collect::<Vec<_>>(),
        );
    }
}