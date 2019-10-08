use dusk_poseidon_merkle::{Scalar, MERKLE_ARITY};
use std::io::{self, Write};

pub fn generate(t: usize) -> Vec<Vec<Scalar>> {
    let mut matrix: Vec<Vec<Scalar>> = Vec::with_capacity(t);
    let mut xs: Vec<Scalar> = Vec::with_capacity(t);
    let mut ys: Vec<Scalar> = Vec::with_capacity(t);

    // Generate x and y values deterministically for the cauchy matrix
    // where x[i] != y[i] to allow the values to be inverted
    // and there are no duplicates in the x vector or y vector, so that the determinant is always non-zero
    // [a b]
    // [c d]
    // det(M) = (ad - bc) ; if a == b and c == d => det(M) =0
    // For an MDS matrix, every possible mxm submatrix, must have det(M) != 0
    for i in 0..t {
        let x = Scalar::from((i) as u64);
        let y = Scalar::from((i + t) as u64);
        xs.push(x);
        ys.push(y);
    }

    for i in 0..t {
        let mut row: Vec<Scalar> = Vec::with_capacity(t);
        for j in 0..t {
            // Generate the entry at (i,j)
            let entry = (xs[i] + ys[j]).invert();
            row.insert(j, entry);
        }
        matrix.push(row);
    }

    matrix
}

fn main() {
    let mds = generate(MERKLE_ARITY + 1);
    let mds = mds.into_iter().flatten().fold(vec![], |mut v, scalars| {
        v.extend_from_slice(scalars.as_bytes());
        v
    });

    io::stdout().write_all(mds.as_slice()).unwrap();
}
