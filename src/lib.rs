#![feature(external_doc)]
#![deny(missing_docs)]
#![doc(include = "../README.md")]

pub use crate::poseidon::Poseidon;
pub use curve25519_dalek::scalar::Scalar;
pub use error::Error;
use lazy_static::*;
pub use merkle::MerkleTree;
use std::ops;

mod error;
mod merkle;
mod poseidon;

// Poseidon constants
pub(crate) const WIDTH: usize = 5;
pub(crate) const FULL_ROUNDS: usize = 8;
pub(crate) const PARTIAL_ROUNDS: usize = 59;

// Merkle constants
/// Arity of the merkle tree
pub const MERKLE_ARITY: usize = 4;
/// Width of the merkle tree
pub const MERKLE_WIDTH: usize = 64;
pub(crate) const _MERKLE_HEIGHT: usize = 4;

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
    Copy + From<u64> + From<Scalar> + PartialEq + ops::MulAssign + ops::AddAssign
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
        assert!(_MERKLE_HEIGHT > 2);

        // Enforce a relation between the provided MDS matrix and the arity of the merkle tree
        assert_eq!(WIDTH, MERKLE_ARITY + 1);

        // Enforce at least one level for the merkle tree
        assert!(MERKLE_WIDTH > MERKLE_ARITY);

        // Grant the defined arity is consistent with the defined width
        assert_eq!(
            MERKLE_ARITY.pow(std::cmp::max(2, _MERKLE_HEIGHT as u32 - 1)),
            MERKLE_WIDTH
        );
    }
}
