use criterion::{criterion_group, criterion_main, Criterion};
use dusk_poseidon_merkle::*;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::time::Duration;

fn bench_merkle_proof_verify(c: &mut Criterion) {
    let mut t = MerkleTree::<Scalar>::default();
    for i in 0..MERKLE_WIDTH {
        t.insert_unchecked(i, Scalar::from(OsRng.next_u64()));
    }
    let leaves = *t.leaves();

    c.bench_function("merkle", |b| {
        b.iter(|| {
            let leaf = leaves.choose(&mut OsRng).unwrap().unwrap();
            let root = t.clone().root();
            t.clone()
                .proof(&leaf)
                .map(|p| p.verify(&leaf, &root))
                .unwrap();
        })
    });
}

criterion_group! {
    name = merkle;
    config = Criterion::default().sample_size(40).measurement_time(Duration::from_secs(60));
    targets = bench_merkle_proof_verify
}
criterion_main!(merkle);
