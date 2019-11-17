#[cfg(feature = "zkproof")]
use crate::{CompressedRistretto, Error, R1CSProof};
use crate::{Poseidon, PoseidonLeaf, Scalar, MERKLE_ARITY};

use serde::Serialize;
use std::ops;

#[cfg(feature = "zkproof")]
use bulletproofs::r1cs::{ConstraintSystem, LinearCombination, Prover, Variable, Verifier};
#[cfg(feature = "zkproof")]
use bulletproofs::{BulletproofGens, PedersenGens};
#[cfg(feature = "zkproof")]
use merlin::Transcript;
#[cfg(feature = "zkproof")]
use rand::rngs::OsRng;
#[cfg(feature = "zkproof")]
use rand::RngCore;

#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct BigProofItem<T: PoseidonLeaf> {
    idx: usize,
    leaves: [Option<T>; MERKLE_ARITY],
}

impl<T: PoseidonLeaf> BigProofItem<T> {
    pub fn new(idx: usize, leaves: [Option<T>; MERKLE_ARITY]) -> Self {
        BigProofItem { idx, leaves }
    }

    pub fn idx(&self) -> &usize {
        &self.idx
    }

    pub fn leaves(&self) -> &[Option<T>; MERKLE_ARITY] {
        &self.leaves
    }
}

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
#[derive(Serialize, Debug, Clone)]
pub struct BigProof<T: PoseidonLeaf> {
    data: Vec<BigProofItem<T>>,

    #[cfg(feature = "zkproof")]
    r1cs_proof: Option<R1CSProof>,

    #[cfg(feature = "zkproof")]
    commitments: Vec<CompressedRistretto>,
}

impl<T: PoseidonLeaf> PartialEq for BigProof<T> {
    fn eq(&self, rhs: &Self) -> bool {
        // ZkProofs are non-deterministic
        self.data.eq(&rhs.data)
    }
}

impl<T: PoseidonLeaf> Default for BigProof<T> {
    fn default() -> Self {
        BigProof::new(vec![])
    }
}

#[cfg(feature = "zkproof")]
impl<T: PoseidonLeaf>
    From<(
        Vec<BigProofItem<T>>,
        Option<R1CSProof>,
        Vec<CompressedRistretto>,
    )> for BigProof<T>
{
    fn from(
        args: (
            Vec<BigProofItem<T>>,
            Option<R1CSProof>,
            Vec<CompressedRistretto>,
        ),
    ) -> Self {
        let (data, r1cs_proof, commitments) = args;

        BigProof {
            data,
            r1cs_proof,
            commitments,
        }
    }
}

impl<T: PoseidonLeaf> BigProof<T> {
    /// BigProof constructor
    pub fn new(data: Vec<BigProofItem<T>>) -> Self {
        BigProof {
            data,

            #[cfg(feature = "zkproof")]
            r1cs_proof: None,

            #[cfg(feature = "zkproof")]
            commitments: vec![],
        }
    }

    pub(crate) fn push(&mut self, idx: usize, leaves: [Option<T>; MERKLE_ARITY]) {
        self.data.push(BigProofItem::new(idx, leaves))
    }

    /// Return the raw proof data
    pub fn data(&self) -> &Vec<BigProofItem<T>> {
        &self.data
    }

    /// Recreate the root based on the proof
    pub fn root(&self) -> T
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        let leaf = self.data[0];
        let mut leaf = leaf.leaves()[*leaf.idx()].unwrap_or(T::from(0u64));

        let mut h = Poseidon::default();

        self.data.iter().for_each(|item| {
            let idx = item.idx();
            let data = item.leaves();

            h.replace(&data[0..MERKLE_ARITY]);
            h.insert_unchecked(*idx, leaf);

            leaf = h.hash();
        });

        leaf
    }

    /// Verify if the provided leaf corresponds to the proof in the merkle construction
    pub fn verify(&self, leaf: &T, root: &T) -> bool
    where
        Scalar: ops::Mul<T, Output = T>,
    {
        let mut leaf = *leaf;
        let mut h = Poseidon::default();

        self.data.iter().for_each(|item| {
            let idx = item.idx();
            let data = item.leaves();

            h.replace(&data[0..MERKLE_ARITY]);
            h.insert_unchecked(*idx, leaf);

            leaf = h.hash();
        });

        &leaf == root
    }

    #[cfg(feature = "zkproof")]
    /// Fetch a tuple with the zk proof. This function will return an error if there is no zk
    /// proof information available.
    ///
    /// The zk proof is generated via the `zk_proof` method.
    pub fn zk(&self) -> Result<(&R1CSProof, &[CompressedRistretto]), Error> {
        let proof = self
            .r1cs_proof
            .as_ref()
            .ok_or(Error::Other("No R1CS proof provided.".to_owned()))?;

        let commitments = self.commitments.as_slice();

        Ok((proof, commitments))
    }

    #[cfg(feature = "zkproof")]
    /// Clone into a new instance with the required zk information
    pub fn clone_zk(&mut self) -> Result<Self, Error> {
        self.zk_proof()?;
        Ok(self.clone())
    }

    #[cfg(feature = "zkproof")]
    /// Calculate and store the zero knowledge proof + commitments
    pub fn zk_proof(&mut self) -> Result<(), Error> {
        let idx = *self.data[0].idx();

        let set: Vec<Scalar> = self.data[0]
            .leaves()
            .iter()
            .map(|leaf| leaf.map(|l| l.into()).unwrap_or(Scalar::one()))
            .collect();

        let (pc_gens, bp_gens, mut transcript) = gen_cs_transcript();

        let mut commitments = vec![];
        let mut variables = vec![];
        let mut bits = vec![];

        let mut prover = Prover::new(&pc_gens, &mut transcript);

        set.iter()
            .enumerate()
            .fold(Ok(()), |status: Result<(), Error>, (i, _)| {
                status?;

                let bit = if i == idx {
                    Scalar::one()
                } else {
                    Scalar::zero()
                };

                let blinding = gen_random_scalar();
                let (commitment, variable) = prover.commit(bit, blinding);

                bit_gadget(&mut prover, variable, Some(bit))?;

                commitments.push(commitment);
                variables.push(variable);
                bits.push(bit);

                Ok(())
            })?;

        sum_is_one_gadget(&mut prover, variables.as_slice())?;

        let blinding = gen_random_scalar();
        let (commitment, variable) = prover.commit(set[idx].into(), blinding);
        commitments.push(commitment);

        values_bitmasked_is_value_gadget(
            &mut prover,
            set.as_slice(),
            Some(bits.as_slice()),
            &variable,
        )?;

        let proof = prover
            .prove(&bp_gens)
            .map_err(|e| Error::Other(e.to_string()))?;

        self.commitments = commitments;
        self.r1cs_proof = Some(proof);

        Ok(())
    }

    #[cfg(feature = "zkproof")]
    /// Verify if the provided proof is correct
    pub fn zk_verify(&self) -> Result<(), Error> {
        let set: Vec<Scalar> = self.data[0]
            .leaves()
            .iter()
            .map(|leaf| leaf.map(|l| l.into()).unwrap_or(Scalar::one()))
            .collect();

        let (proof, commitments) = self.zk()?;

        let (pc_gens, bp_gens, mut transcript) = gen_cs_transcript();
        let mut verifier = Verifier::new(&mut transcript);

        let mut variables = vec![];

        set.iter()
            .enumerate()
            .fold(Ok(()), |status: Result<(), Error>, (i, _)| {
                status?;

                let variable = verifier.commit(commitments[i]);
                bit_gadget(&mut verifier, variable, None)?;
                variables.push(variable);

                Ok(())
            })?;

        sum_is_one_gadget(&mut verifier, variables.as_slice())?;

        let variable = verifier.commit(commitments[set.len()]);
        values_bitmasked_is_value_gadget(&mut verifier, set.as_slice(), None, &variable)?;

        verifier
            .verify(proof, &pc_gens, &bp_gens)
            .map_err(|e| Error::Other(e.to_string()))
    }
}

#[cfg(feature = "zkproof")]
/// Grant the provided variable is 1 or 0
fn bit_gadget<C: ConstraintSystem>(
    cs: &mut C,
    var: Variable,
    assignment: Option<Scalar>,
) -> Result<(), Error> {
    let (a, b, o) = cs
        .allocate_multiplier(assignment.map(|a| (Scalar::one() - a, a)))
        .map_err(|e| Error::Other(e.to_string()))?;

    let neg_var = vec![(var, -Scalar::one())]
        .iter()
        .collect::<LinearCombination>();

    cs.constrain(b + neg_var);
    cs.constrain(o.into());
    cs.constrain(a + (b - 1u64));

    Ok(())
}

#[cfg(feature = "zkproof")]
/// Grant the sum of the variables is 1
fn sum_is_one_gadget<C: ConstraintSystem>(cs: &mut C, variables: &[Variable]) -> Result<(), Error> {
    cs.constrain(
        variables
            .iter()
            .map(|v| (*v, Scalar::one()))
            .fold(vec![(Variable::One(), -Scalar::one())], |mut vars, pair| {
                vars.push(pair);
                vars
            })
            .iter()
            .collect::<LinearCombination>(),
    );

    Ok(())
}

#[cfg(feature = "zkproof")]
/// Grant the sum of the product between the values and the bitflags is the initial value;
/// therefore, the bitflag position is synchronized with the correspondent value position
fn values_bitmasked_is_value_gadget<C: ConstraintSystem>(
    cs: &mut C,
    set: &[Scalar],
    bits: Option<&[Scalar]>,
    var: &Variable,
) -> Result<(), Error> {
    let mut constraints = vec![(*var, -Scalar::one())];

    for i in 0..set.len() {
        let (a, b, o) = cs
            .allocate_multiplier(bits.map(|b| (b[i], set[i])))
            .map_err(|e| Error::Other(e.to_string()))?;

        let lc_b: LinearCombination = b.into();
        let lc_item = LinearCombination::from(set[i]);

        cs.constrain(lc_b - lc_item);

        let (_, _, o2) = cs.multiply(a.into(), (*var).into());

        cs.constrain(o - o2);

        constraints.push((o, Scalar::one()));
    }

    cs.constrain(constraints.iter().collect());
    Ok(())
}

#[cfg(feature = "zkproof")]
/// Generate a random Scalar to be used as blinding factor
fn gen_random_scalar() -> Scalar {
    let mut s = [0x00u8; 32];
    OsRng.fill_bytes(&mut s);
    Scalar::from_bits(s)
}

#[cfg(feature = "zkproof")]
/// Generate the constraint system and the transcript for the zk proofs
fn gen_cs_transcript() -> (PedersenGens, BulletproofGens, Transcript) {
    let pc_gens = PedersenGens::default();
    let bp_gens = BulletproofGens::new(128, 1);
    let transcript = Transcript::new(b"big-merkle-bp");

    (pc_gens, bp_gens, transcript)
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

    #[test]
    #[cfg(feature = "zkproof")]
    fn big_proof_zk() {
        let mut t = big_merkle_default("big_proof_verify_failure");
        for i in 0..64 {
            t.insert(i, Scalar::from(i as u64)).unwrap();
        }

        let mut proof: BigProof<Scalar> = t.proof(21).unwrap();

        proof
            .clone_zk()
            .and_then(|new_proof| new_proof.zk_verify())
            .unwrap();
    }
}
