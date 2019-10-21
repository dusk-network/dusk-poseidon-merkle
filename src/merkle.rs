use crate::{
    Error, Poseidon, PoseidonLeaf, Proof, Scalar, MERKLE_ARITY, MERKLE_HEIGHT, MERKLE_WIDTH,
};
use std::ops;

/// The merkle tree will accept up to `MERKLE_ARITY * MERKLE_WIDTH` leaves.
#[derive(Copy, Clone)]
pub struct MerkleTree<T: PoseidonLeaf> {
    root: Option<T>,
    leaves: [Option<T>; MERKLE_WIDTH],
    raw: [[Option<T>; MERKLE_WIDTH]; MERKLE_HEIGHT + 1],
}

impl<T: PoseidonLeaf> Default for MerkleTree<T> {
    fn default() -> Self {
        MerkleTree {
            raw: [[None; MERKLE_WIDTH]; MERKLE_HEIGHT + 1],
            root: None,
            leaves: [None; MERKLE_WIDTH],
        }
    }
}

impl<T: PoseidonLeaf> MerkleTree<T> {
    /// Return a reference to the provided leaves
    pub fn leaves(&self) -> &[Option<T>; MERKLE_WIDTH] {
        &self.leaves
    }

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

    /// Generate a proof of membership for the provided leaf
    pub fn proof(&mut self, leaf: &T) -> Result<Proof<T>, Error>
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        self.leaves
            .iter()
            .enumerate()
            .fold(None, |mut idx, (i, il)| {
                if let Some(l) = il {
                    if idx.is_none() && l == leaf {
                        idx.replace(i);
                    }
                }

                idx
            })
            .ok_or(Error::LeafNotFound)
            .map(|i| self.proof_index(i))
    }

    /// Generate a proof of membership for the provided leaf index
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn proof_index(&mut self, mut needle: usize) -> Proof<T>
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        self.root();
        let mut proof = Proof::default();

        for row in 0..MERKLE_HEIGHT {
            let from = MERKLE_ARITY * (needle / MERKLE_ARITY);
            let to = from + MERKLE_ARITY;
            let idx = needle % MERKLE_ARITY;

            proof.push(idx, &self.raw[row][from..to]);
            needle /= MERKLE_ARITY;
        }

        proof
    }

    /// Calculate and return the root of the merkle tree.
    pub fn root(&mut self) -> T
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        if let Some(s) = self.root {
            return s;
        }

        self.raw[0].copy_from_slice(&self.leaves);
        for i in 1..self.raw.len() {
            self.raw[i].copy_from_slice(&[None; MERKLE_WIDTH]);
        }

        let mut merkle = MERKLE_WIDTH;
        let mut h = Poseidon::default();

        for raw_index in 1..MERKLE_HEIGHT + 1 {
            for i in (0..merkle).step_by(MERKLE_ARITY) {
                let from = i;
                let to = i + MERKLE_ARITY;
                let idx = to / MERKLE_ARITY - 1;

                h.replace(&self.raw[raw_index - 1][from..to]);
                self.raw[raw_index][idx] = Some(h.hash());
            }

            merkle /= MERKLE_ARITY;
        }

        self.root = self.raw[MERKLE_HEIGHT][0];
        match self.root {
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

        for _ in 0..MERKLE_HEIGHT - 1 {
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
