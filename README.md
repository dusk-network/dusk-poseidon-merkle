# Poseidon Merkle Tree

[![Build Status](https://travis-ci.com/dusk-network/dusk-poseidon-merkle.svg?branch=master)](https://travis-ci.com/dusk-network/dusk-poseidon-merkle)
[![Repository](https://dusk-network.github.io/dusk-poseidon-merkle/repo-badge.svg)](https://github.com/dusk-network/dusk-poseidon-merkle)
[![Documentation](https://dusk-network.github.io/dusk-poseidon-merkle/badge.svg)](https://dusk-network.github.io/dusk-poseidon-merkle/dusk_poseidon_merkle/index.html)

Reference implementation for the Poseidon Merkle function.

The `Poseidon` structure will accept a number of inputs equal to the arity of the tree.

## Example

```rust
use dusk_poseidon_merkle::{MERKLE_ARITY, Poseidon, Scalar};

let mut h = Poseidon::default();
for i in 0..MERKLE_ARITY {
    h.push(Scalar::from(i as u64)).unwrap();
}

let hash = h.hash();
println!("{:x?}", hash.as_bytes());
```

## Reference

[Starkad and Poseidon: New Hash Functions for Zero Knowledge Proof Systems](https://eprint.iacr.org/2019/458.pdf)
