use criterion::{criterion_group, criterion_main, Criterion};
use dusk_poseidon_merkle::*;
use lazy_static::*;
use std::env;
use std::time::Duration;

// 2^36
const WIDTH: usize = 68719476736;

lazy_static! {
    static ref POSEIDON_BENCH_BIG_MERKLE_SAMPLE_SIZE: usize = {
        env::var("POSEIDON_BENCH_BIG_MERKLE_SAMPLE_SIZE")
            .map(|s| {
                s.parse()
                    .expect("Failed to parse POSEIDON_BENCH_BIG_MERKLE_SAMPLE_SIZE")
            })
            .unwrap_or(10)
    };
    static ref POSEIDON_BENCH_BIG_MERKLE_MEASUREMENT_TIME: Duration = {
        Duration::from_secs(
            env::var("POSEIDON_BENCH_BIG_MERKLE_MEASUREMENT_TIME")
                .map(|s| {
                    s.parse()
                        .expect("Failed to parse POSEIDON_BENCH_BIG_MERKLE_MEASUREMENT_TIME")
                })
                .unwrap_or(60),
        )
    };
}

fn bench_big_merkle(c: &mut Criterion) {
    let mut group = c.benchmark_group("big_merkle");

    let iter = vec![10, 1000];
    for x in iter {
        let path = format!("big_merkle_{}", x);
        let desc = format!(
            "Proof with width {}, arity {}, elements {}",
            WIDTH, MERKLE_ARITY, x
        );
        let mut tree: BigMerkleTree<Scalar> = BigMerkleTree::new(path.as_str(), WIDTH).unwrap();
        for i in 0..10 {
            tree.insert(i, Scalar::from(i as u64)).unwrap();
        }
        group.bench_function(desc.as_str(), move |b| b.iter(|| proof(&mut tree)));
    }

    group.finish();
}

fn proof(tree: &mut BigMerkleTree<Scalar>) {
    tree.clear_cache(false).unwrap();
    tree.proof(0).unwrap();
}

criterion_group! {
    name = big_merkle;

    config = Criterion::default()
        .sample_size(*POSEIDON_BENCH_BIG_MERKLE_SAMPLE_SIZE)
        .measurement_time(*POSEIDON_BENCH_BIG_MERKLE_MEASUREMENT_TIME);

    targets = bench_big_merkle
}
criterion_main!(big_merkle);
