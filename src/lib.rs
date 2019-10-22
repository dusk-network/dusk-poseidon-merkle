#![feature(external_doc)]
#![deny(missing_docs)]
#![doc(include = "../README.md")]

pub use crate::poseidon::Poseidon;
pub use curve25519_dalek::scalar::Scalar;
pub use error::Error;
use lazy_static::*;
pub use merkle::MerkleTree;
pub use proof::Proof;
use std::ops;

mod error;
mod merkle;
mod poseidon;
mod proof;

include!("constants.rs");

lazy_static! {
    static ref ROUND_CONSTANTS: [Scalar; 960] = {
        let bytes = include_bytes!("../assets/ark.bin");
        unsafe { std::ptr::read(bytes.as_ptr() as *const _) }
    };
    static ref MDS_MATRIX: [[Scalar; WIDTH]; WIDTH] = {
        let bytes = include_bytes!("../assets/mds.bin");
        assert_eq!(bytes.len(), (WIDTH * WIDTH) << 5);
        unsafe { std::ptr::read(bytes.as_ptr() as *const _) }
    };
}

/// The items for the [`MerkleTree`] and [`Poseidon`] must implement this trait
pub trait PoseidonLeaf:
    Copy + From<u64> + From<Scalar> + PartialEq + ops::MulAssign + ops::AddAssign + Send + Sync
{
}
impl PoseidonLeaf for Scalar {}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn constants_consistency() {
        // Grant we have enough constants for the sbox rounds
        assert!(WIDTH * (FULL_ROUNDS + PARTIAL_ROUNDS) <= ROUND_CONSTANTS.len());

        // Sanity check for the arity
        assert!(MERKLE_ARITY > 1);

        // Sanity check for the height
        assert!(MERKLE_HEIGHT > 1);

        // Enforce a relation between the provided MDS matrix and the arity of the merkle tree
        assert_eq!(WIDTH, MERKLE_ARITY + 1);

        // Enforce at least one level for the merkle tree
        assert!(MERKLE_WIDTH > MERKLE_ARITY);

        // Grant the defined arity is consistent with the defined width
        assert_eq!(
            MERKLE_ARITY.pow(std::cmp::max(2, MERKLE_HEIGHT as u32)),
            MERKLE_WIDTH
        );
    }
}
