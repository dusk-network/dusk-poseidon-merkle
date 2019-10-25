use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dusk_poseidon_merkle::*;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use sha2::{Digest, Sha256, Sha512};

fn bench_hash(c: &mut Criterion) {
    let scalars: Vec<Scalar> = std::iter::repeat(())
        .take(1000)
        .enumerate()
        .map(|(i, _)| Scalar::from(i as u64))
        .collect();

    let mut group = c.benchmark_group("hash");

    group.bench_with_input(
        BenchmarkId::new("Sha2 256", "Generated scalars"),
        &scalars,
        |b, s| {
            b.iter(|| {
                let mut h = Sha256::new();

                std::iter::repeat(())
                    .take(MERKLE_ARITY)
                    .map(|_| s.choose(&mut OsRng).unwrap())
                    .for_each(|scalar| {
                        h.input(scalar.as_bytes());
                    });

                h.result();
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("Sha2 512", "Generated scalars"),
        &scalars,
        |b, s| {
            b.iter(|| {
                let mut h = Sha512::new();

                std::iter::repeat(())
                    .take(MERKLE_ARITY)
                    .map(|_| s.choose(&mut OsRng).unwrap())
                    .for_each(|scalar| {
                        h.input(scalar.as_bytes());
                    });

                h.result();
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("Poseidon hash", "Generated scalars"),
        &scalars,
        |b, s| {
            b.iter(|| {
                let mut h = Poseidon::default();

                std::iter::repeat(())
                    .take(MERKLE_ARITY)
                    .map(|_| s.choose(&mut OsRng).unwrap())
                    .for_each(|scalar| {
                        h.push(*scalar).unwrap();
                    });

                h.hash();
            })
        },
    );

    group.finish();
}

criterion_group! {
    name = hash;

    config = Criterion::default();

    targets = bench_hash
}
criterion_main!(hash);
