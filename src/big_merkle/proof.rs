use crate::{Poseidon, PoseidonLeaf, Scalar, MERKLE_ARITY};

use std::ops;

/// Set of pairs (idx, Hash) to reconstruct the merkle root.
/// For every level of the tree,
/// Required information to reconstruct the merkle root.
///
/// For every level of the tree, there is an index, and a slice of leaves.
///
/// The index will be the position in which the previously calculated information should be
/// inserted.
///
/// The leaves will define the other elements required to perform the hash for that level of the
/// tree.
#[derive(Debug, Clone, PartialEq)]
pub struct BigProof<T: PoseidonLeaf> {
    data: Vec<(usize, [Option<T>; MERKLE_ARITY])>,
}

impl<T: PoseidonLeaf> BigProof<T> {
    /// BigProof constructor
    pub fn new() -> Self {
        BigProof { data: vec![] }
    }

    pub(crate) fn push(&mut self, idx: usize, leaves: [Option<T>; MERKLE_ARITY]) {
        self.data.push((idx, leaves))
    }

    /// Return the raw proof data
    pub fn data(&self) -> &Vec<(usize, [Option<T>; MERKLE_ARITY])> {
        &self.data
    }

    /// Verify if the provided leaf corresponds to the proof in the merkle construction
    pub fn verify(&self, leaf: &T, root: &T) -> bool
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        let mut leaf = *leaf;
        let mut h = Poseidon::default();

        self.data.iter().for_each(|(idx, data)| {
            h.replace(&data[0..MERKLE_ARITY]);
            h.insert_unchecked(*idx, leaf);

            leaf = h.hash();
        });

        &leaf == root
    }
}

#[cfg(test)]
mod tests {
    use super::super::big_merkle_default;
    use crate::*;

    #[test]
    fn big_proof_verify() {
        let mut t = big_merkle_default("big_proof_verify");
        for i in 0..64 {
            t.insert(i, Scalar::from(i as u64)).unwrap();
        }

        let root = t.root().unwrap();
        let i = 21;

        let proof = t.proof(i).unwrap();
        assert!(proof.verify(&Scalar::from(i as u64), &root));
    }

    #[test]
    fn big_proof_verify_failure() {
        let mut t = big_merkle_default("big_proof_verify_failure");
        for i in 0..64 {
            t.insert(i, Scalar::from(i as u64)).unwrap();
        }

        let root = t.root().unwrap();
        let i = 21;

        let proof = t.proof(i + 1).unwrap();
        assert!(!proof.verify(&Scalar::from(i as u64), &root));
    }
}
