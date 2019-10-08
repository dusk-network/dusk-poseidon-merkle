use crate::{Poseidon, PoseidonLeaf, Scalar, MERKLE_ARITY, MERKLE_WIDTH};
use std::ops;

/// The merkle tree will accept up to `MERKLE_ARITY * MERKLE_WIDTH` leaves.
#[derive(Copy, Clone)]
pub struct MerkleTree<T: PoseidonLeaf> {
    root: Option<T>,
    leaves: [Option<T>; MERKLE_WIDTH],
}

impl<T: PoseidonLeaf> Default for MerkleTree<T> {
    fn default() -> Self {
        MerkleTree {
            root: None,
            leaves: [None; MERKLE_WIDTH],
        }
    }
}

impl<T: PoseidonLeaf> MerkleTree<T> {
    /// Insert the provided leaf in the defined position.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn insert_unchecked(&mut self, index: usize, leaf: T) {
        self.root = None;
        self.leaves[index].replace(leaf);
    }

    /// Set the provided leaf index as absent for the hash calculation.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove_unchecked(&mut self, index: usize) -> Option<T> {
        self.root = None;
        self.leaves[index].take()
    }

    /// Calculate and return the root of the merkle tree.
    pub fn root(&mut self) -> T
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        if let Some(s) = self.root {
            return s;
        }

        let mut leaves: [Option<T>; MERKLE_WIDTH] = [None; MERKLE_WIDTH];
        leaves.copy_from_slice(&self.leaves);

        let mut merkle = MERKLE_WIDTH;
        let mut h = Poseidon::default();
        while merkle > 1 {
            for i in (0..merkle).step_by(MERKLE_ARITY) {
                let from = i;
                let to = i + MERKLE_ARITY;
                let idx = to / MERKLE_ARITY - 1;

                h.replace(&leaves[from..to]);
                leaves[idx] = Some(h.hash());
            }

            merkle /= MERKLE_ARITY;
        }

        self.root = leaves[0];
        match leaves[0] {
            Some(s) => s,
            None => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn merkle() {
        let mut t = MerkleTree::default();
        t.insert_unchecked(0, Scalar::one());
        let root = t.root();
        assert_ne!(Scalar::zero(), root)
    }

    #[test]
    fn merkle_pad() {
        let mut t = MerkleTree::default();
        t.insert_unchecked(0, Scalar::one());
        let root = t.root();

        let mut t = MerkleTree::default();
        t.insert_unchecked(0, Scalar::one());
        t.insert_unchecked(1, Scalar::zero());

        assert_ne!(t.root(), root)
    }

    #[test]
    fn merkle_det() {
        let mut v = vec![];
        for i in 0..MERKLE_ARITY {
            v.push(Scalar::from(i as u64));
        }

        let mut t = MerkleTree::default();
        v.iter()
            .enumerate()
            .for_each(|(i, s)| t.insert_unchecked(i, *s));
        let root = t.root();

        let mut t = MerkleTree::default();
        v.iter()
            .enumerate()
            .for_each(|(i, s)| t.insert_unchecked(i, *s));

        assert_eq!(t.root(), root)
    }

    #[test]
    fn merkle_sanity_proof() {
        let base = Scalar::one();
        let mut t = MerkleTree::default();
        t.insert_unchecked(0, base);

        let root = t.root();

        let mut h = Poseidon::default();
        h.push(base).unwrap();
        let mut main_path = h.hash();

        h.reset();
        let mut round_void = h.hash();
        let mut void: Vec<Option<Scalar>> = std::iter::repeat(Some(round_void))
            .take(MERKLE_ARITY)
            .collect();

        for _ in 0.._MERKLE_HEIGHT - 2 {
            h.replace(void.as_slice());
            round_void = h.hash();

            void[0] = Some(main_path);
            h.replace(void.as_slice());
            main_path = h.hash();

            void = std::iter::repeat(Some(round_void))
                .take(MERKLE_ARITY)
                .collect();
        }

        assert_eq!(root, main_path);
    }
}
