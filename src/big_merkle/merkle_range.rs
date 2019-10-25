use crate::MERKLE_ARITY;

use std::cmp::Ordering;
use std::ops::Range;

/// Struct to represent a range in the base of the tree
#[derive(Debug, Eq, Clone)]
pub struct MerkleRange(pub Range<usize>);

impl Ord for MerkleRange {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.start.cmp(&other.0.start)
    }
}

impl PartialOrd for MerkleRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MerkleRange {
    /// The equivalence for a merkle range is defined by any provided range that is contained
    /// within the current range.
    ///
    /// Therefore, this is not commutative, and should be used with care.
    fn eq(&self, other: &Self) -> bool {
        self.0.start <= other.0.start && self.0.end >= other.0.end
    }
}

impl From<Range<usize>> for MerkleRange {
    fn from(r: Range<usize>) -> Self {
        MerkleRange(r)
    }
}

impl MerkleRange {
    /// Will return a range within the max row for a relative merkle position
    pub fn new(max_height: usize, height: usize, idx: usize) -> Self {
        let h = max_height - height;
        let h = h as u32;

        let from = MERKLE_ARITY.pow(h) * idx;
        let to = MERKLE_ARITY.pow(h) * (idx + 1);

        MerkleRange::from(from..to)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn big_merkle_range_eq() {
        let r1 = MerkleRange::new(3, 0, 0);
        let r2 = MerkleRange::new(3, 3, 15);

        assert!(r1 == r2);
        assert!(r2 != r1)
    }
}
