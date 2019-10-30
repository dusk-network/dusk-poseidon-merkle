use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dusk_poseidon_merkle::*;
use lazy_static::*;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::env;
use std::time::Duration;

lazy_static! {
    static ref POSEIDON_BENCH_MERKLE_SAMPLE_SIZE: usize = {
        env::var("POSEIDON_BENCH_MERKLE_SAMPLE_SIZE")
            .map(|s| {
                s.parse()
                    .expect("Failed to parse POSEIDON_BENCH_MERKLE_SAMPLE_SIZE")
            })
            .unwrap_or(20)
    };
    static ref POSEIDON_BENCH_MERKLE_MEASUREMENT_TIME: Duration = {
        Duration::from_secs(
            env::var("POSEIDON_BENCH_MERKLE_MEASUREMENT_TIME")
                .map(|s| {
                    s.parse()
                        .expect("Failed to parse POSEIDON_BENCH_MERKLE_MEASUREMENT_TIME")
                })
                .unwrap_or(15),
        )
    };
}

fn bench_merkle(c: &mut Criterion) {
    let mut tree = MerkleTree::<Scalar>::default();
    for i in 0..MERKLE_WIDTH {
        tree.insert_unchecked(i, Scalar::from(OsRng.next_u64()));
    }

    let leaves = *tree.leaves();
    let root = tree.clone().root();
    let mut temp_tree = tree.clone();
    let proofs: Vec<(Scalar, Proof<Scalar>)> = leaves
        .iter()
        .map(|l| (l.unwrap(), temp_tree.proof(&l.unwrap()).unwrap()))
        .collect();

    let mut group = c.benchmark_group("merkle");

    group.bench_with_input(
        BenchmarkId::new("Proof", "Generated tree"),
        &(leaves.clone(), tree.clone()),
        |b, (l, t)| {
            b.iter(|| {
                let leaf = l.choose(&mut OsRng).unwrap().unwrap();
                t.clone().proof(&leaf).unwrap();
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("Verify", "Generated proofs"),
        &(root.clone(), proofs.clone()),
        |b, (r, p)| {
            b.iter(|| {
                let (leaf, proof) = p.choose(&mut OsRng).unwrap();
                proof.verify(&leaf, r);
            })
        },
    );

    group.finish();
}

criterion_group! {
    name = merkle;

    config = Criterion::default()
        .sample_size(*POSEIDON_BENCH_MERKLE_SAMPLE_SIZE)
        .measurement_time(*POSEIDON_BENCH_MERKLE_MEASUREMENT_TIME);

    targets = bench_merkle
}
criterion_main!(merkle);
